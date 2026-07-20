//! RDB-backed chapter repository step implementations.

use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use tracing::instrument;

use crate::model::chapter::{
    ChapterEntry, ChapterInfo, ChapterInfoListSpec, ChapterInfoUpdate,
    ChapterStageUpdate,
};
use crate::model::unit::UnitCounterDelta;
use crate::part_impl::repo::rdb_impl::entity::chapter::{
    ChapterAspect, ChapterRow, ChapterRowEntry,
};
use crate::part_impl::repo::rdb_impl::incl;
use crate::part_impl::repo::rdb_impl::schema::t_chapter::dsl::*;
use crate::part_impl::repo::rdb_impl::schema::t_page;
use crate::part_impl::shared::RdbConn;
use crate::part_impl::shared::result::{diesel, expected};
use crate::result::{BaseError, BaseResult, accept};
use crate::value::chapter::{ChapterInclOpt, Stage};

/// Converts a single `ChapterRow` into a `ChapterInfo`.
fn row_into_info(row: ChapterRow) -> BaseResult<ChapterInfo> {
    row.try_into()
}

/// Converts a vector of `ChapterRow` values into `ChapterInfo`.
fn rows_into_infos(rows: Vec<ChapterRow>) -> BaseResult<Vec<ChapterInfo>> {
    rows.into_iter().map(row_into_info).collect()
}

/// Queries a single chapter row by ID and populates its includes.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> BaseResult<ChapterInfo> {
    //
    let row: ChapterRow = t_chapter
        .filter(f_id.eq(id))
        .select(ChapterRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-chapter-not-found"))?;

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
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> BaseResult<ChapterInfo> {
    //
    let row: ChapterRow = t_chapter
        .filter(f_id.eq(id))
        .select(ChapterRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-chapter-not-found"))?;

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
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos(
    conn: &mut RdbConn,
    spec: &ChapterInfoListSpec,
) -> BaseResult<Vec<ChapterInfo>> {
    //
    let rows: Vec<ChapterRow> = t_chapter
        .filter(f_comic_id.eq(spec.comic_id.as_str()))
        .select(ChapterRow::as_select())
        .order_by(f_index.desc())
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    let mut infos = rows_into_infos(rows)?;

    incl::chapter::populate_chapter_incls(conn, &mut infos, &spec.incl_opt)
        .await?;

    accept(infos)
}

/// Queries all chapter rows for a comic under `FOR UPDATE` lock.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos_excluded(
    conn: &mut RdbConn,
    comic_id: &str,
) -> BaseResult<Vec<ChapterInfo>> {
    //
    let rows: Vec<ChapterRow> = t_chapter
        .filter(f_comic_id.eq(comic_id))
        .select(ChapterRow::as_select())
        .order_by(f_index.desc())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    rows_into_infos(rows)
}

/// Finds the pinned chapter for a given comic ID, if one exists.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn find_pinned_info_by_comic_id(
    conn: &mut RdbConn,
    comic_id: &str,
    incl_opt: &[ChapterInclOpt],
) -> BaseResult<Option<ChapterInfo>> {
    //
    let row: Option<ChapterRow> = t_chapter
        .filter(f_comic_id.eq(comic_id))
        .filter(f_is_pinned.eq(true))
        .select(ChapterRow::as_select())
        .get_result(conn)
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

/// Returns a map of comic ID to pinned chapter info for the given comic IDs.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_pinned_infos_by_comic_ids(
    conn: &mut RdbConn,
    comic_ids: &[String],
) -> BaseResult<HashMap<String, ChapterInfo>> {
    //
    if comic_ids.is_empty() {
        return accept(HashMap::new());
    }

    let rows: Vec<ChapterRow> = t_chapter
        .filter(f_comic_id.eq_any(comic_ids))
        .filter(f_is_pinned.eq(true))
        .select(ChapterRow::as_select())
        .load(conn)
        .await
        .map_err(diesel)?;

    let mut map = HashMap::with_capacity(rows.len());

    for row in rows {
        //
        let info = row_into_info(row)?;

        map.insert(info.comic_id.clone(), info);
    }

    accept(map)
}

/// Inserts a new chapter row from the given entry and returns the created info.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create(
    conn: &mut RdbConn,
    chapter_entry: &ChapterEntry,
) -> BaseResult<ChapterInfo> {
    //
    let entry = ChapterRowEntry::from(chapter_entry);

    let row: ChapterRow = diesel::insert_into(t_chapter)
        .values(&entry)
        .returning(ChapterRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    row_into_info(row)
}

/// Updates the modifiable fields of a chapter row.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn update_info(
    conn: &mut RdbConn,
    update: &ChapterInfoUpdate,
) -> BaseResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let mut aspect = ChapterAspect::new(now);

    if let Some(subtitle) = &update.subtitle {
        aspect = aspect.subtitle(subtitle);
    }

    if let Some(pin) = update.pin {
        aspect = aspect.pinned(pin);
    }

    diesel::update(t_chapter.filter(f_id.eq(update.id.as_str())))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Updates the stage timestamps of a chapter row.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn update_stage(
    conn: &mut RdbConn,
    update: &ChapterStageUpdate,
) -> BaseResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = ChapterAspect::new(now).stages(update.stages, now);

    diesel::update(t_chapter.filter(f_id.eq(update.id.as_str())))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Atomically moves a pending two-step stage to active.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn start_stage(
    conn: &mut RdbConn,
    id: &str,
    stage: Stage,
) -> BaseResult<bool> {
    //
    let now = OffsetDateTime::now_utc();

    let updated_count = match stage {
        //
        Stage::Translate => diesel::update(
            t_chapter
                .filter(f_id.eq(id))
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
                .filter(f_typesetting_at.is_null())
                .filter(f_typeset_at.is_null()),
        )
        .set((f_typesetting_at.eq(now), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?,

        Stage::RawProvide | Stage::Review | Stage::Publish => {
            return Err(BaseError::Unrecoverable {
                message: "only two-step chapter stages can be started"
                    .to_string(),
            });
        }
    };

    accept(updated_count > 0)
}

/// Atomically completes raw provision when every reserved page is uploaded.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn complete_raw_provide(
    conn: &mut RdbConn,
    id: &str,
) -> BaseResult<bool> {
    //
    let now = OffsetDateTime::now_utc();

    let incomplete_pages = t_page::table
        .filter(t_page::f_chapter_id.eq(id))
        .filter(t_page::f_image_uploaded.eq(false));

    let updated_count = diesel::update(
        t_chapter
            .filter(f_id.eq(id))
            .filter(f_uploaded_at.is_null())
            .filter(f_page_count.gt(0))
            .filter(diesel::dsl::not(diesel::dsl::exists(incomplete_pages))),
    )
    .set((f_uploaded_at.eq(now), f_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(updated_count > 0)
}

/// Sets the page and unit counters on a chapter row.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn set_page_counters(
    conn: &mut RdbConn,
    id: &str,
    page_count: i32,
    total_unit_count: i32,
    translated_unit_count: i32,
    proofread_unit_count: i32,
) -> BaseResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = ChapterAspect::new(now)
        .page_count(page_count)
        .total_unit_count(total_unit_count)
        .translated_unit_count(translated_unit_count)
        .proofread_unit_count(proofread_unit_count);

    diesel::update(t_chapter.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Adjusts a chapter's unit counters by the given delta.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn adjust_unit_counters(
    conn: &mut RdbConn,
    id: &str,
    delta: &UnitCounterDelta,
) -> BaseResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    diesel::update(t_chapter.filter(f_id.eq(id)))
        .set((
            f_total_unit_count.eq(f_total_unit_count + delta.total_unit_count),
            f_translated_unit_count
                .eq(f_translated_unit_count + delta.translated_unit_count),
            f_proofread_unit_count
                .eq(f_proofread_unit_count + delta.proofread_unit_count),
            f_updated_at.eq(now),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Unpins all chapters for a comic except the one with the given excluded ID.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn unpin_others(
    conn: &mut RdbConn,
    comic_id: &str,
    excluded_id: &str,
) -> BaseResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    diesel::update(
        t_chapter
            .filter(f_comic_id.eq(comic_id))
            .filter(f_id.ne(excluded_id)),
    )
    .set((f_is_pinned.eq(false), f_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

/// Deletes a single chapter row by ID.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    //
    diesel::delete(t_chapter.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}
