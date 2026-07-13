//! RDB-backed chapter repository.

use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use crate::model::chapter::{
    ChapterEntry, ChapterInfo, ChapterInfoListSpec, ChapterInfoUpdate,
    ChapterStageUpdate,
};
use crate::model::unit::UnitCounterDelta;
use crate::part::repo::chapter::ChapterRepo;
use crate::part_impl::repo::rdb_impl::entity::chapter::{
    ChapterAspect, ChapterRow, ChapterRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_chapter::dsl::*;
use crate::part_impl::repo::rdb_impl::{RdbRepo, incl};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::RegularResult;
use crate::value::chapter::ChapterInclOpt;

impl ChapterRepo<RdbContext> for RdbRepo {}

mod orchestra;

/// Converts a single `ChapterRow` into a `ChapterInfo`.
fn row_into_info(row: ChapterRow) -> RegularResult<ChapterInfo> {
    row.try_into()
}

/// Converts a vector of `ChapterRow` values into `ChapterInfo`.
fn rows_into_infos(rows: Vec<ChapterRow>) -> RegularResult<Vec<ChapterInfo>> {
    rows.into_iter().map(row_into_info).collect()
}

/// Queries a single chapter row by ID and populates its includes.
pub(super) async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> RegularResult<ChapterInfo> {
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

    Ok(info)
}

/// Queries a single chapter row by ID under `FOR UPDATE` lock.
pub(super) async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> RegularResult<ChapterInfo> {
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

    Ok(info)
}

/// Queries chapter rows for a given comic, ordered by index descending.
pub(super) async fn list_infos(
    conn: &mut RdbConn,
    spec: &ChapterInfoListSpec,
) -> RegularResult<Vec<ChapterInfo>> {
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

    Ok(infos)
}

/// Queries all chapter rows for a comic under `FOR UPDATE` lock.
pub(super) async fn list_infos_excluded(
    conn: &mut RdbConn,
    comic_id: &str,
) -> RegularResult<Vec<ChapterInfo>> {
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
pub(super) async fn find_pinned_info_by_comic_id(
    conn: &mut RdbConn,
    comic_id: &str,
    incl_opt: &[ChapterInclOpt],
) -> RegularResult<Option<ChapterInfo>> {
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
        return Ok(None);
    };

    let mut info = row_into_info(row)?;

    incl::chapter::populate_chapter_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    Ok(Some(info))
}

/// Returns a map of comic ID to pinned chapter info for the given comic IDs.
pub(super) async fn list_pinned_infos_by_comic_ids(
    conn: &mut RdbConn,
    comic_ids: &[String],
) -> RegularResult<HashMap<String, ChapterInfo>> {
    //
    if comic_ids.is_empty() {
        return Ok(HashMap::new());
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

    Ok(map)
}

/// Inserts a new chapter row from the given entry and returns the created info.
pub(super) async fn create(
    conn: &mut RdbConn,
    chapter_entry: &ChapterEntry,
) -> RegularResult<ChapterInfo> {
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
pub(super) async fn update_info(
    conn: &mut RdbConn,
    update: &ChapterInfoUpdate,
) -> RegularResult<()> {
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

    Ok(())
}

/// Updates the stage timestamps of a chapter row.
pub(super) async fn update_stage(
    conn: &mut RdbConn,
    update: &ChapterStageUpdate,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = ChapterAspect::new(now).stages(update.stages, now);

    diesel::update(t_chapter.filter(f_id.eq(update.id.as_str())))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Sets the page and unit counters on a chapter row.
pub(super) async fn set_page_counters(
    conn: &mut RdbConn,
    id: &str,
    page_count: i32,
    total_unit_count: i32,
    translated_unit_count: i32,
    proofread_unit_count: i32,
) -> RegularResult<()> {
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

    Ok(())
}

/// Adjusts a chapter's unit counters by the given delta.
pub(super) async fn adjust_unit_counters(
    conn: &mut RdbConn,
    id: &str,
    delta: &UnitCounterDelta,
) -> RegularResult<()> {
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

    Ok(())
}

/// Unpins all chapters for a comic except the one with the given excluded ID.
pub(super) async fn unpin_others(
    conn: &mut RdbConn,
    comic_id: &str,
    excluded_id: &str,
) -> RegularResult<()> {
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

    Ok(())
}

/// Deletes a single chapter row by ID.
pub(super) async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    //
    diesel::delete(t_chapter.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

#[cfg(all(test, feature = "repo"))]
mod tests;
