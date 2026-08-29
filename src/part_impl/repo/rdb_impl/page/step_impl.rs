//! RDB-backed page repository step implementations.

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitCountMetrics;
use crate::model::write::page::{PageEntry, PageManifestRepl};
use crate::part_impl::repo::rdb_impl::entity::page::{
    PageAspectRow, PageEntryRow, PageInfoRow,
};
use crate::part_impl::repo::rdb_impl::numeric::i32_from_usize;
use crate::part_impl::repo::rdb_impl::schema::t_page::dsl::{
    f_chapter_id, f_id, f_index, f_updated_at, t_page,
};
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_page_id as unit_f_page_id, t_unit,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;

/// Load a single page info by ID.
#[instrument(level = "info", skip_all)]
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<PageInfo> {
    //
    let row = t_page
        .filter(f_id.eq(id))
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

/// Load a page info by ID, locking the row for update.
#[instrument(level = "info", skip_all)]
pub async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<PageInfo> {
    //
    let row = t_page
        .filter(f_id.eq(id))
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
        .order_by(f_index.asc())
        .load::<PageInfoRow>(conn)
        .await
        .map_err(diesel)?;

    rows.into_iter().map(TryInto::try_into).collect()
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
        .for_update()
        .load::<PageInfoRow>(conn)
        .await
        .map_err(diesel)?;

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

/// Persists the final index and image identity for one manifest page.
#[instrument(level = "info", skip_all)]
pub async fn update_manifest(
    conn: &mut RdbConn,
    update: &PageManifestRepl,
) -> BaseRest<PageInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let row = diesel::update(t_page.filter(f_id.eq(&update.id)))
        .set((
            f_index.eq(i32_from_usize(update.index, "t_page.f_index")?),
            f_updated_at.eq(now),
        ))
        .returning(PageInfoRow::as_returning())
        .get_result::<PageInfoRow>(conn)
        .await
        .map_err(diesel)?;

    row.try_into()
}

/// Query the lowest-index page info for each requested chapter.
#[instrument(level = "info", skip_all)]
pub async fn list_first_infos_by_chapter_ids(
    conn: &mut RdbConn,
    chapter_ids: &[String],
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

/// Batch-insert pages from a slice of `model_entries` and return the created infos.
#[instrument(level = "info", skip_all)]
pub async fn create_batch(
    conn: &mut RdbConn,
    model_entries: &[PageEntry],
) -> BaseRest<Vec<PageInfo>> {
    //
    let entries = model_entries
        .iter()
        .map(PageEntryRow::try_from)
        .collect::<BaseRest<Vec<PageEntryRow>>>()?;

    let rows = diesel::insert_into(t_page)
        .values(&entries)
        .returning(PageInfoRow::as_returning())
        .get_results::<PageInfoRow>(conn)
        .await
        .map_err(diesel)?;

    rows.into_iter().map(TryInto::try_into).collect()
}

/// Persist unit counters (total, translated, proofread) onto a page row.
#[instrument(level = "info", skip_all)]
pub async fn set_unit_counters(
    conn: &mut RdbConn,
    id: &str,
    counters: UnitCountMetrics,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = PageAspectRow::new(now)
        .total_unit_count(i32_from_usize(
            counters.total,
            "t_page.f_total_unit_count",
        )?)
        .translated_unit_count(i32_from_usize(
            counters.translated,
            "t_page.f_translated_unit_count",
        )?)
        .proofread_unit_count(i32_from_usize(
            counters.proofread,
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
        diesel::delete(t_unit.filter(unit_f_page_id.eq_any(&page_ids)))
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

    diesel::delete(t_unit.filter(unit_f_page_id.eq_any(ids)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    diesel::delete(t_page.filter(f_id.eq_any(ids)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}
