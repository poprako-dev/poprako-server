//! RDB-backed page repository.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use tracing::instrument;

use crate::complex::page::PageComplex;
use crate::model::page::{PageEntry, PageImageReservation, PageInfo};
use crate::model::unit::UnitCounters;
use crate::part::repo::page::PageRepo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::page::{
    PageAspect, PageRow, PageRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_page::dsl::*;
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_page_id as unit_f_page_id, t_unit,
};
use crate::part_impl::shared::result::{diesel, expected, version};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::RegularResult;

impl PageRepo<RdbContext> for RdbRepo {}

mod orchestra;

/// Load a single page info by ID.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<PageInfo> {
    //
    let row: PageRow = t_page
        .filter(f_id.eq(id))
        .select(PageRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-page-not-found"))?;

    Ok(row.into())
}

/// Load a page info by ID, locking the row for update.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<PageInfo> {
    //
    let row: PageRow = t_page
        .filter(f_id.eq(id))
        .select(PageRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-page-not-found"))?;

    Ok(row.into())
}

/// Query a paginated list of page infos for a chapter, ordered by index.
#[instrument(level = "info", err(Debug), skip_all)]
async fn list_infos_by_chapter_id(
    conn: &mut RdbConn,
    chapter_id: &str,
    offset: u32,
    limit: u32,
) -> RegularResult<Vec<PageInfo>> {
    //
    let rows: Vec<PageRow> = t_page
        .filter(f_chapter_id.eq(chapter_id))
        .select(PageRow::as_select())
        .order_by(f_index.asc())
        .offset(offset as i64)
        .limit(limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Query all page infos for a chapter, ordered by index (no pagination).
#[instrument(level = "info", err(Debug), skip_all)]
async fn list_all_infos_by_chapter_id(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> RegularResult<Vec<PageInfo>> {
    //
    let rows: Vec<PageRow> = t_page
        .filter(f_chapter_id.eq(chapter_id))
        .select(PageRow::as_select())
        .order_by(f_index.asc())
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Batch-insert pages from a slice of model_entries and return the created infos.
#[instrument(level = "info", err(Debug), skip_all)]
async fn create_batch(
    conn: &mut RdbConn,
    model_entries: &[PageEntry],
) -> RegularResult<Vec<PageInfo>> {
    //
    let entries: Vec<PageRowEntry> =
        model_entries.iter().map(PageRowEntry::from).collect();

    let rows: Vec<PageRow> = diesel::insert_into(t_page)
        .values(&entries)
        .returning(PageRow::as_returning())
        .get_results(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Reserve a new image slot for a page: bump version, generate object key,
/// and return the reservation with previous key for cleanup.
#[instrument(level = "info", err(Debug), skip_all)]
async fn reserve_image(
    conn: &mut RdbConn,
    id: &str,
    file_ext: &str,
) -> RegularResult<PageImageReservation> {
    //
    let now = OffsetDateTime::now_utc();

    let (chapter_id, prev_key, raw_version): (String, Option<String>, i64) =
        diesel::update(t_page.filter(f_id.eq(id)))
            .set((
                f_image_key.eq::<Option<&str>>(None),
                f_image_uploaded.eq(false),
                f_image_version.eq(f_image_version + 1),
                f_updated_at.eq(now),
            ))
            .returning((f_chapter_id, f_image_key, f_image_version))
            .get_result(conn)
            .await
            .map_err(diesel)?;

    let image_version = version(raw_version)?;

    let object_key =
        PageComplex::gen_image_key(&chapter_id, id, image_version, file_ext);

    let aspect = PageAspect::new(now).image_key(Some(&object_key));

    diesel::update(t_page.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(PageImageReservation {
        object_key,
        prev_object_key: prev_key,
        image_version,
    })
}

/// Mark a page's image as successfully uploaded, checking version staleness.
#[instrument(level = "info", err(Debug), skip_all)]
async fn mark_image_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: u32,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let affected = diesel::update(
        t_page
            .filter(f_id.eq(id))
            .filter(f_image_version.eq(i64::from(version))),
    )
    .set((f_image_uploaded.eq(true), f_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    if affected == 0 {
        return Err(expected("error-stale-page-image-upload"));
    }

    Ok(())
}

/// Persist unit counters (total, translated, proofread) onto a page row.
#[instrument(level = "info", err(Debug), skip_all)]
async fn set_unit_counters(
    conn: &mut RdbConn,
    id: &str,
    counters: UnitCounters,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = PageAspect::new(now)
        .total_unit_count(counters.total_unit_count)
        .translated_unit_count(counters.translated_unit_count)
        .proofread_unit_count(counters.proofread_unit_count);

    diesel::update(t_page.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Delete all pages (and their child units) for a given chapter.
#[instrument(level = "info", err(Debug), skip_all)]
async fn delete_by_chapter_id(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> RegularResult<()> {
    //
    let page_ids: Vec<String> = t_page
        .filter(f_chapter_id.eq(chapter_id))
        .select(f_id)
        .load(conn)
        .await
        .map_err(diesel)?;

    if !page_ids.is_empty() {
        diesel::delete(t_unit.filter(unit_f_page_id.eq_any(&page_ids)))
            .execute(conn)
            .await
            .map_err(diesel)?;
    }

    diesel::delete(t_page.filter(f_chapter_id.eq(chapter_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

#[cfg(all(test, feature = "repo"))]
mod tests;
