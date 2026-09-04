//! RDB-backed chapter repository step implementations.

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::unit::UnitCountDelta;
use crate::model::read::spec::chapter::ChapterListSpec;
use crate::model::write::chapter::{
    ChapterEntry, ChapterPatch, ChapterStageRepl,
};
use crate::part_impl::repo::rdb_impl::entity::chapter::{
    ChapterAspectRow, ChapterEntryRow, ChapterInfoRow,
};
use crate::part_impl::repo::rdb_impl::incl;
use crate::part_impl::repo::rdb_impl::schema::t_chapter::dsl::{
    f_comic_id, f_deleted_at, f_id, f_index, f_is_pinned, f_page_count,
    f_proofread_at, f_proofread_unit_count, f_proofreading_at, f_published_at,
    f_total_unit_count, f_translated_at, f_translated_unit_count,
    f_translating_at, f_typeset_at, f_typesetting_at, f_updated_at,
    f_uploaded_at, t_chapter,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;
use crate::value::chapter::ChapterInclOpt;
use crate::value::chapter::stage::Stage;

/// Build the expected error for a missing chapter.
pub fn missing_chapter(id: &str, operation: &str) -> BaseError {
    //
    let err_message = trl("error-chapter-not-found");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        chapter_id = %id,
        operation,
        "expected error: chapter not found",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
}

/// Queries a single chapter row by ID and populates its includes.
#[instrument(level = "info", skip_all)]
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> BaseRest<ChapterInfo> {
    //
    let row = t_chapter
        .filter(f_id.eq(id))
        .filter(f_deleted_at.is_null())
        .select(ChapterInfoRow::as_select())
        .get_result::<ChapterInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            let err_message = trl("error-chapter-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                chapter_id = %id,
                stage = "get_info_by_id",
                "expected error: chapter not found",
            );

            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            }
        })?;

    let mut info = row_into_info(row)?;

    incl::chapter::populate_chapter_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    accept(info)
}

/// Queries a single chapter row by ID under `FOR UPDATE` lock.
#[instrument(level = "info", skip_all)]
pub async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> BaseRest<ChapterInfo> {
    //
    let row = t_chapter
        .filter(f_id.eq(id))
        .filter(f_deleted_at.is_null())
        .select(ChapterInfoRow::as_select())
        .for_update()
        .get_result::<ChapterInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            let err_message = trl("error-chapter-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                chapter_id = %id,
                stage = "get_info_excluded",
                "expected error: chapter not found",
            );

            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            }
        })?;

    let mut info = row_into_info(row)?;

    incl::chapter::populate_chapter_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    accept(info)
}

/// Queries chapter rows for a given comic, ordered by index descending.
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    conn: &mut RdbConn,
    spec: &ChapterListSpec,
) -> BaseRest<Vec<ChapterInfo>> {
    //
    let rows = t_chapter
        .filter(f_comic_id.eq(spec.comic_id.as_str()))
        .filter(f_deleted_at.is_null())
        .select(ChapterInfoRow::as_select())
        .order_by(f_index.desc())
        .offset(i64::from(spec.offset))
        .limit(i64::from(spec.limit.get()))
        .load::<ChapterInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let mut infos = rows_into_infos(rows)?;

    incl::chapter::populate_chapter_incls(conn, &mut infos, &spec.incl_opt)
        .await?;

    accept(infos)
}

/// Queries all chapter rows for a comic under `FOR UPDATE` lock.
#[instrument(level = "info", skip_all)]
pub async fn list_infos_excluded(
    conn: &mut RdbConn,
    comic_id: &str,
) -> BaseRest<Vec<ChapterInfo>> {
    //
    let rows = t_chapter
        .filter(f_comic_id.eq(comic_id))
        .filter(f_deleted_at.is_null())
        .select(ChapterInfoRow::as_select())
        .order_by(f_index.desc())
        .for_update()
        .load::<ChapterInfoRow>(conn)
        .await
        .map_err(diesel)?;

    rows_into_infos(rows)
}

/// Locks all chapter rows belonging to a comic.
#[instrument(level = "info", skip_all)]
pub async fn lock_chapters(conn: &mut RdbConn, comic_id: &str) -> BaseRest<()> {
    //
    let _ = t_chapter
        .filter(f_comic_id.eq(comic_id))
        .filter(f_deleted_at.is_null())
        .select(f_id)
        .for_update()
        .load::<String>(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Finds the pinned chapter for a given comic ID, if one exists.
#[instrument(level = "info", skip_all)]
pub async fn find_pinned_info_by_comic_id(
    conn: &mut RdbConn,
    comic_id: &str,
    incl_opt: &[ChapterInclOpt],
) -> BaseRest<Option<ChapterInfo>> {
    //
    let row = t_chapter
        .filter(f_comic_id.eq(comic_id))
        .filter(f_deleted_at.is_null())
        .filter(f_is_pinned.eq(true))
        .select(ChapterInfoRow::as_select())
        .get_result::<ChapterInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        return accept(None);
    };

    let mut info = row_into_info(row)?;

    incl::chapter::populate_chapter_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    accept(Some(info))
}

/// Returns the pinned chapter infos for the given comic IDs.
#[instrument(level = "info", skip_all)]
pub async fn list_pinned_infos_by_comic_ids(
    conn: &mut RdbConn,
    comic_ids: &[&str],
) -> BaseRest<Vec<ChapterInfo>> {
    //
    if comic_ids.is_empty() {
        return accept(Vec::new());
    }

    let rows = t_chapter
        .filter(f_comic_id.eq_any(comic_ids))
        .filter(f_deleted_at.is_null())
        .filter(f_is_pinned.eq(true))
        .select(ChapterInfoRow::as_select())
        .load::<ChapterInfoRow>(conn)
        .await
        .map_err(diesel)?;

    rows.into_iter().map(row_into_info).collect()
}

/// Inserts a new chapter row from the given entry and returns the created info.
#[instrument(level = "info", skip_all)]
pub async fn create(
    conn: &mut RdbConn,
    chapter_entry: &ChapterEntry,
) -> BaseRest<ChapterInfo> {
    //
    let entry = ChapterEntryRow::try_from(chapter_entry)?;

    let row = diesel::insert_into(t_chapter)
        .values(&entry)
        .returning(ChapterInfoRow::as_returning())
        .get_result::<ChapterInfoRow>(conn)
        .await
        .map_err(diesel)?;

    row_into_info(row)
}

/// Updates the modifiable fields of a chapter row.
#[instrument(level = "info", skip_all)]
pub async fn update_info(
    conn: &mut RdbConn,
    update: &ChapterPatch,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let mut aspect = ChapterAspectRow::new(now);

    if let Some(subtitle) = &update.subtitle {
        aspect = aspect.subtitle(subtitle);
    }

    if let Some(pin) = update.pin {
        aspect = aspect.pinned(pin);
    }

    let updated_count = diesel::update(
        t_chapter
            .filter(f_id.eq(update.id.as_str()))
            .filter(f_deleted_at.is_null()),
    )
    .set(&aspect)
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_count == 0 {
        return Err(missing_chapter(&update.id, "update chapter info"));
    }

    accept(())
}

/// Updates the stage timestamps of a chapter row.
#[instrument(level = "info", skip_all)]
pub async fn update_stage(
    conn: &mut RdbConn,
    update: &ChapterStageRepl,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = ChapterAspectRow::new(now).stages(update.stages, now);

    let updated_count = diesel::update(
        t_chapter
            .filter(f_id.eq(update.id.as_str()))
            .filter(f_deleted_at.is_null()),
    )
    .set(&aspect)
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_count == 0 {
        return Err(missing_chapter(&update.id, "update chapter stage"));
    }

    accept(())
}

/// Atomically moves a pending two-step stage to active.
#[instrument(level = "info", skip_all)]
pub async fn start_stage(
    conn: &mut RdbConn,
    id: &str,
    stage: Stage,
) -> BaseRest<bool> {
    //
    let now = OffsetDateTime::now_utc();

    let updated_count = match stage {
        //
        Stage::Translate => diesel::update(
            t_chapter
                .filter(f_id.eq(id))
                .filter(f_deleted_at.is_null())
                .filter(f_published_at.is_null())
                .filter(f_translating_at.is_null())
                .filter(f_translated_at.is_null()),
        )
        .set((f_translating_at.eq(now), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?,

        Stage::Proofread => diesel::update(
            t_chapter
                .filter(f_id.eq(id))
                .filter(f_deleted_at.is_null())
                .filter(f_published_at.is_null())
                .filter(f_proofreading_at.is_null())
                .filter(f_proofread_at.is_null()),
        )
        .set((f_proofreading_at.eq(now), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?,

        Stage::TypesetRedraw => diesel::update(
            t_chapter
                .filter(f_id.eq(id))
                .filter(f_deleted_at.is_null())
                .filter(f_published_at.is_null())
                .filter(f_typesetting_at.is_null())
                .filter(f_typeset_at.is_null()),
        )
        .set((f_typesetting_at.eq(now), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?,

        Stage::RawProvide | Stage::Review | Stage::Publish => {
            //
            return Err(BaseError::Unrecoverable {
                message: "only two-step chapter stages can be started"
                    .to_string(),
            });
        }
    };

    accept(updated_count > 0)
}

/// Atomically resolves raw provision when every reserved page is uploaded.
///
/// Missing and already completed chapters are resolved idempotently. A
/// pending chapter returns `false` while at least one upload is incomplete.
#[instrument(level = "info", skip_all)]
pub async fn complete_raw_provide(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<bool> {
    //
    let now = OffsetDateTime::now_utc();

    let updated_count = diesel::update(
        t_chapter
            .filter(f_id.eq(id))
            .filter(f_deleted_at.is_null())
            .filter(f_uploaded_at.is_null())
            .filter(f_page_count.gt(0)),
    )
    .set((f_uploaded_at.eq(now), f_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(updated_count > 0)
}

/// Sets the page and unit counters on a chapter row.
#[instrument(level = "info", skip_all)]
pub async fn set_page_counters(
    conn: &mut RdbConn,
    id: &str,
    page_count: i32,
    total_unit_count: i32,
    translated_unit_count: i32,
    proofread_unit_count: i32,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = ChapterAspectRow::new(now)
        .page_count(page_count)
        .total_unit_count(total_unit_count)
        .translated_unit_count(translated_unit_count)
        .proofread_unit_count(proofread_unit_count);

    let updated_count = diesel::update(
        t_chapter.filter(f_id.eq(id)).filter(f_deleted_at.is_null()),
    )
    .set(&aspect)
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_count == 0 {
        return Err(missing_chapter(id, "set chapter page counters"));
    }

    accept(())
}

/// Adjusts a chapter's unit counters by the given delta.
#[instrument(level = "info", skip_all)]
pub async fn adjust_unit_counters(
    conn: &mut RdbConn,
    id: &str,
    delta: &UnitCountDelta,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let updated_count = diesel::update(
        t_chapter.filter(f_id.eq(id)).filter(f_deleted_at.is_null()),
    )
    .set((
        f_total_unit_count.eq(f_total_unit_count + delta.total),
        f_translated_unit_count.eq(f_translated_unit_count + delta.translated),
        f_proofread_unit_count.eq(f_proofread_unit_count + delta.proofread),
        f_updated_at.eq(now),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_count == 0 {
        return Err(missing_chapter(id, "adjust chapter unit counters"));
    }

    accept(())
}

/// Unpins all chapters for a comic except the one with the given excluded ID.
#[instrument(level = "info", skip_all)]
pub async fn unpin_others(
    conn: &mut RdbConn,
    comic_id: &str,
    excluded_id: &str,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    diesel::update(
        t_chapter
            .filter(f_comic_id.eq(comic_id))
            .filter(f_deleted_at.is_null())
            .filter(f_id.ne(excluded_id)),
    )
    .set((f_is_pinned.eq(false), f_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

// Converts a single `ChapterInfoRow` into a `ChapterInfo`.
fn row_into_info(row: ChapterInfoRow) -> BaseRest<ChapterInfo> {
    row.try_into()
}

// Converts a vector of `ChapterInfoRow` values into `ChapterInfo`.
fn rows_into_infos(rows: Vec<ChapterInfoRow>) -> BaseRest<Vec<ChapterInfo>> {
    rows.into_iter().map(row_into_info).collect()
}
