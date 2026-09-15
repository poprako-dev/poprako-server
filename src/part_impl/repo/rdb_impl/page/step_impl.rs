//! RDB-backed page repository step implementations.

use diesel::PgExpressionMethods as _;
use diesel::dsl::{case_when, count};
use diesel::expression::functions::declare_sql_function;
use diesel::prelude::{
    BoolExpressionMethods as _, ExpressionMethods as _, JoinOnDsl as _,
    NullableExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel::sql_types::{Nullable, Text};
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::page::{
    PageInfo, PageUnitDiffStats, PageUnitScope,
};
use crate::model::read::proj::unit::UnitCountMetrics;
use crate::model::write::page::PageManifestEntry;
use crate::part_impl::repo::rdb_impl::entity::page::{
    PageAspectRow, PageEntryRow, PageInfoRow, PageUnitScopeRow,
};
use crate::part_impl::repo::rdb_impl::numeric::{
    i32_from_usize, usize_from_i32, usize_from_i64,
};
use crate::part_impl::repo::rdb_impl::schema::t_chapter;
use crate::part_impl::repo::rdb_impl::schema::t_page::dsl::{
    f_chapter_id, f_id, f_index, f_updated_at, t_page,
};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_hidden_at as unit_hidden_at, f_page_id as unit_page_id,
    f_proofread_text as unit_proofread_text,
    f_translated_text as unit_translated_text, t_unit,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;
use crate::value::page::{CHAPTER_PAGE_SENTINEL_LIMIT, MAX_CHAPTER_PAGE_COUNT};

#[declare_sql_function]
extern "SQL" {
    /// `PostgreSQL` text trim with an explicit character set.
    // PostgreSQL text trim with an explicit character set.
    fn btrim(string: Nullable<Text>, characters: Text) -> Nullable<Text>;
}

// `char::is_whitespace` code points accepted by Unit text semantics.
const UNIT_TEXT_WHITESPACE: &str = "\u{0009}\u{000A}\u{000B}\u{000C}\u{000D}\u{0020}\u{0085}\u{00A0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}";

/// Load a single page info by ID.
#[instrument(level = "info", skip_all)]
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<PageInfo> {
    //
    let row = t_page
        .filter(f_id.eq(id))
        .filter(
            f_chapter_id.eq_any(
                t_chapter::table
                    .filter(t_chapter::f_deleted_at.is_null())
                    .select(t_chapter::f_id),
            ),
        )
        .select(PageInfoRow::as_select())
        .get_result::<PageInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            let err_message = trl("error-page-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                page_id = %id,
                stage = "get_info_by_id",
                "expected error: page not found",
            );

            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            }
        })?;

    row.try_into()
}

/// Loads the minimal Page scope used by Unit operations.
#[instrument(level = "info", skip_all)]
pub async fn get_unit_scope(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<PageUnitScope> {
    //
    let row = t_page
        .filter(f_id.eq(id))
        .filter(
            f_chapter_id.eq_any(
                t_chapter::table
                    .filter(t_chapter::f_deleted_at.is_null())
                    .select(t_chapter::f_id),
            ),
        )
        .select(PageUnitScopeRow::as_select())
        .get_result::<PageUnitScopeRow>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| missing_page(id, "get_unit_scope"))?;

    row.try_into()
}

/// Locks and loads the minimal Page scope used by Unit edits.
#[instrument(level = "info", skip_all)]
pub async fn get_unit_scope_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<PageUnitScope> {
    //
    let row = t_page
        .filter(f_id.eq(id))
        .filter(
            f_chapter_id.eq_any(
                t_chapter::table
                    .filter(t_chapter::f_deleted_at.is_null())
                    .select(t_chapter::f_id),
            ),
        )
        .select(PageUnitScopeRow::as_select())
        .for_update()
        .get_result::<PageUnitScopeRow>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| missing_page(id, "get_unit_scope_excluded"))?;

    row.try_into()
}

/// Load a page info by ID, locking the row for update.
#[instrument(level = "info", skip_all)]
pub async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<PageInfo> {
    //
    let row = t_page
        .filter(f_id.eq(id))
        .filter(
            f_chapter_id.eq_any(
                t_chapter::table
                    .filter(t_chapter::f_deleted_at.is_null())
                    .select(t_chapter::f_id),
            ),
        )
        .select(PageInfoRow::as_select())
        .for_update()
        .get_result::<PageInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            let err_message = trl("error-page-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                page_id = %id,
                stage = "get_info_excluded",
                "expected error: page not found",
            );

            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            }
        })?;

    row.try_into()
}

/// Queries page infos for a chapter, ordered by index.
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<Vec<PageInfo>> {
    //
    let rows = t_page
        .filter(f_chapter_id.eq(chapter_id))
        .select(PageInfoRow::as_select())
        .order_by((f_index.asc(), f_id.asc()))
        .limit(CHAPTER_PAGE_SENTINEL_LIMIT)
        .load::<PageInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let rows = ensure_chapter_page_count(rows, chapter_id, "list_infos")?;

    rows.into_iter().map(TryInto::try_into).collect()
}

/// Lists text statistics for Chapter Pages with visible revision differences.
#[instrument(level = "info", skip_all)]
pub async fn list_unit_diff_stats(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<Vec<PageUnitDiffStats>> {
    //
    let has_translation =
        btrim(unit_translated_text, UNIT_TEXT_WHITESPACE).ne("");

    let has_revision = btrim(unit_proofread_text, UNIT_TEXT_WHITESPACE).ne("");

    let has_edit = has_translation
        .and(has_revision)
        .and(unit_proofread_text.is_distinct_from(unit_translated_text));

    let has_append = unit_translated_text
        .is_null()
        .or(btrim(unit_translated_text, UNIT_TEXT_WHITESPACE).eq(""))
        .and(has_revision);

    // Retain zero-diff Pages until the complete Chapter count is checked.
    let rows = t_page
        .filter(f_chapter_id.eq(chapter_id))
        .left_join(
            t_unit.on(unit_page_id.eq(f_id).and(unit_hidden_at.is_null())),
        )
        .group_by((f_id, f_index))
        .select((
            f_id,
            f_index,
            count(case_when(has_translation, unit_page_id).nullable()),
            count(case_when(has_edit, unit_page_id).nullable()),
            count(case_when(has_append, unit_page_id).nullable()),
        ))
        .order_by((f_index.asc(), f_id.asc()))
        .limit(CHAPTER_PAGE_SENTINEL_LIMIT)
        .load::<(String, i32, i64, i64, i64)>(conn)
        .await
        .map_err(diesel)?;

    let rows =
        ensure_chapter_page_count(rows, chapter_id, "list_unit_diff_stats")?;

    let page_unit_diff_stats = rows
        .into_iter()
        .map(|(page_id, index, translated, editted, appended)| {
            //
            accept(PageUnitDiffStats {
                page_id,
                index: usize_from_i32(index, "t_page.f_index")?,
                translated_unit_count: usize_from_i64(
                    translated,
                    "unit_diff_stats.translated_unit_count",
                )?,
                editted_unit_count: usize_from_i64(
                    editted,
                    "unit_diff_stats.editted_unit_count",
                )?,
                proofreader_append_unit_count: usize_from_i64(
                    appended,
                    "unit_diff_stats.proofreader_append_unit_count",
                )?,
            })
        })
        .collect::<BaseRest<Vec<_>>>()?;

    accept(
        page_unit_diff_stats
            .into_iter()
            .filter(|stats| {
                //
                stats.editted_unit_count > 0
                    || stats.proofreader_append_unit_count > 0
            })
            .collect(),
    )
}

/// Lists page infos while retaining row locks for a manifest transaction.
#[instrument(level = "info", skip_all)]
pub async fn list_infos_excluded(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<Vec<PageInfo>> {
    //
    let rows = t_page
        .filter(f_chapter_id.eq(chapter_id))
        .select(PageInfoRow::as_select())
        .order_by((f_index.asc(), f_id.asc()))
        .limit(CHAPTER_PAGE_SENTINEL_LIMIT)
        .for_update()
        .load::<PageInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let rows =
        ensure_chapter_page_count(rows, chapter_id, "list_infos_excluded")?;

    rows.into_iter().map(TryInto::try_into).collect()
}

/// Places every normal page index into the temporary negative range.
#[instrument(level = "info", skip_all)]
pub async fn shift_indexes_temporary(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<()> {
    //
    diesel::update(
        t_page
            .filter(f_chapter_id.eq(chapter_id))
            .filter(f_index.ge(0)),
    )
    .set(f_index.eq(f_index * -1 - 1))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

/// Applies the final manifest with one insert-or-index-update statement.
#[instrument(level = "info", skip_all)]
pub async fn apply_manifest(
    conn: &mut RdbConn,
    model_entries: &[PageManifestEntry],
) -> BaseRest<Vec<PageInfo>> {
    //
    let entries = model_entries
        .iter()
        .map(PageEntryRow::try_from)
        .collect::<BaseRest<Vec<_>>>()?;

    let manifest_upsert = diesel::insert_into(t_page)
        .values(&entries)
        .on_conflict(f_id)
        .do_update()
        .set((
            f_index.eq(excluded(f_index)),
            f_updated_at.eq(excluded(f_updated_at)),
        ));

    let rows = diesel::query_dsl::methods::FilterDsl::filter(
        manifest_upsert,
        f_chapter_id.eq(excluded(f_chapter_id)),
    )
    .returning(PageInfoRow::as_returning())
    .get_results::<PageInfoRow>(conn)
    .await
    .map_err(diesel)?;

    if rows.len() != model_entries.len() {
        //
        return Err(BaseError::Unrecoverable {
            message: "page manifest upsert returned an unexpected row count"
                .into(),
        });
    }

    rows.into_iter().map(TryInto::try_into).collect()
}

/// Query the lowest-index page info for each requested chapter.
#[instrument(level = "info", skip_all)]
pub async fn list_first_infos_by_chapter_ids(
    conn: &mut RdbConn,
    chapter_ids: &[&str],
) -> BaseRest<Vec<PageInfo>> {
    //
    let rows = t_page
        .filter(f_chapter_id.eq_any(chapter_ids))
        .select(PageInfoRow::as_select())
        .distinct_on(f_chapter_id)
        .order_by((f_chapter_id.asc(), f_index.asc()))
        .load::<PageInfoRow>(conn)
        .await
        .map_err(diesel)?;

    rows.into_iter().map(TryInto::try_into).collect()
}

/// Persist unit counters (total, translated, proofread) onto a page row.
#[instrument(level = "info", skip_all)]
pub async fn set_unit_counts(
    conn: &mut RdbConn,
    id: &str,
    counts: UnitCountMetrics,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = PageAspectRow::new(now)
        .total_unit_count(i32_from_usize(
            counts.total,
            "t_page.f_total_unit_count",
        )?)
        .translated_unit_count(i32_from_usize(
            counts.translated,
            "t_page.f_translated_unit_count",
        )?)
        .proofread_unit_count(i32_from_usize(
            counts.proofread,
            "t_page.f_proofread_unit_count",
        )?);

    diesel::update(t_page.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Delete all pages (and their child units) for a given chapter.
#[instrument(level = "info", skip_all)]
pub async fn delete_by_chapter_id(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<()> {
    //
    let page_ids = t_page
        .filter(f_chapter_id.eq(chapter_id))
        .select(f_id)
        .load::<String>(conn)
        .await
        .map_err(diesel)?;

    if !page_ids.is_empty() {
        //
        diesel::delete(t_unit.filter(unit_page_id.eq_any(&page_ids)))
            .execute(conn)
            .await
            .map_err(diesel)?;
    }

    diesel::delete(t_page.filter(f_chapter_id.eq(chapter_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Deletes selected pages after deleting their child units.
#[instrument(level = "info", skip_all)]
pub async fn delete_by_ids(conn: &mut RdbConn, ids: &[String]) -> BaseRest<()> {
    //
    if ids.is_empty() {
        return accept(());
    }

    diesel::delete(t_unit.filter(unit_page_id.eq_any(ids)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    diesel::delete(t_page.filter(f_id.eq_any(ids)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Builds the expected error used when a Page scope cannot be found.
fn missing_page(id: &str, operation: &str) -> BaseError {
    //
    let err_message = trl("error-page-not-found");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        page_id = %id,
        operation,
        "expected error: page not found",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
}

// Reject a persisted Chapter whose Page count exceeds the business invariant.
fn ensure_chapter_page_count<T>(
    rows: Vec<T>,
    chapter_id: &str,
    operation: &'static str,
) -> BaseRest<Vec<T>> {
    //
    if rows.len() > MAX_CHAPTER_PAGE_COUNT {
        //
        tracing::error!(
            chapter_id = %chapter_id,
            page_count_lower_bound = rows.len(),
            max_page_count = MAX_CHAPTER_PAGE_COUNT,
            operation,
            "persisted Chapter Page count exceeds the business maximum",
        );

        return Err(BaseError::Unrecoverable {
            message:
                "persisted Chapter Page count exceeds the business maximum"
                    .into(),
        });
    }

    accept(rows)
}
