//! RDB-backed page repository step implementations.

use std::collections::HashMap;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::page::PageComplex;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitCounters;
use crate::model::write::page::{
    PageEntry, PageImageReservation, PageManifestRepl,
};
use crate::part_impl::repo::rdb_impl::entity::page::{
    PageAspect, PageRow, PageRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_page::dsl::*;
use crate::part_impl::repo::rdb_impl::schema::t_unit::dsl::{
    f_page_id as unit_f_page_id, t_unit,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::RdbConn;
use crate::shared::result::{diesel, next_version};

/// Load a single page info by ID.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<PageInfo> {
    //
    let row: PageRow = t_page
        .filter(f_id.eq(id))
        .select(PageRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            let error_message = trl("error-page-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                error_message = %error_message,
                page_id = %id,
                stage = "get_info_by_id",
                "expected error: page not found",
            );

            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: error_message,
            }
        })?;

    row.try_into()
}

/// Load a page info by ID, locking the row for update.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<PageInfo> {
    //
    let row: PageRow = t_page
        .filter(f_id.eq(id))
        .select(PageRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            let error_message = trl("error-page-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                error_message = %error_message,
                page_id = %id,
                stage = "get_info_excluded",
                "expected error: page not found",
            );

            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: error_message,
            }
        })?;

    row.try_into()
}

/// Queries page infos for a chapter, ordered by index.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<Vec<PageInfo>> {
    //
    let rows: Vec<PageRow> = t_page
        .filter(f_chapter_id.eq(chapter_id))
        .select(PageRow::as_select())
        .order_by(f_index.asc())
        .load(conn)
        .await
        .map_err(diesel)?;

    rows.into_iter().map(TryInto::try_into).collect()
}

/// Lists page infos while retaining row locks for a manifest transaction.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos_excluded(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<Vec<PageInfo>> {
    //
    let rows: Vec<PageRow> = t_page
        .filter(f_chapter_id.eq(chapter_id))
        .select(PageRow::as_select())
        .order_by((f_index.asc(), f_id.asc()))
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    rows.into_iter().map(TryInto::try_into).collect()
}

/// Places every normal page index into the temporary negative range.
#[instrument(level = "info", err(Debug), skip_all)]
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
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn update_manifest(
    conn: &mut RdbConn,
    update: &PageManifestRepl,
) -> BaseRest<PageInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let image_hash = update.image_hash.bytes();

    let row: PageRow = diesel::update(t_page.filter(f_id.eq(&update.id)))
        .set((
            f_index.eq(update.index),
            f_image_key.eq(update.image_key.as_deref()),
            f_image_uploaded.eq(update.is_image_uploaded),
            f_image_version.eq(i64::from(update.image_version)),
            f_image_hash.eq(image_hash.to_vec()),
            f_image_extension.eq(update.image_ext.suffix()),
            f_updated_at.eq(now),
        ))
        .returning(PageRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    row.try_into()
}

/// Invalidates every page image identity after chapter publication.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn clear_images_for_publish(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<Vec<String>> {
    //
    let rows: Vec<PageRow> = t_page
        .filter(f_chapter_id.eq(chapter_id))
        .select(PageRow::as_select())
        .order_by((f_index.asc(), f_id.asc()))
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    let page_infos = rows
        .into_iter()
        .map(TryInto::try_into)
        .collect::<BaseRest<Vec<PageInfo>>>()?;

    let object_keys = page_infos
        .iter()
        .filter_map(|page_info| page_info.image_key.clone())
        .collect::<Vec<_>>();

    let now = OffsetDateTime::now_utc();

    for page_info in page_infos {
        //
        let image_version =
            page_info.image_version.checked_add(1).ok_or_else(|| {
                BaseError::Unrecoverable {
                    message:
                        "[clear_images_for_publish] image version overflow"
                            .into(),
                }
            })?;

        diesel::update(t_page.filter(f_id.eq(&page_info.id)))
            .set((
                f_image_key.eq(None::<String>),
                f_image_uploaded.eq(false),
                f_image_version.eq(i64::from(image_version)),
                f_updated_at.eq(now),
            ))
            .execute(conn)
            .await
            .map_err(diesel)?;
    }

    accept(object_keys)
}

/// Query the lowest-index page info for each requested chapter.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_first_infos_by_chapter_ids(
    conn: &mut RdbConn,
    chapter_ids: &[String],
) -> BaseRest<HashMap<String, PageInfo>> {
    //
    let rows: Vec<PageRow> = t_page
        .filter(f_chapter_id.eq_any(chapter_ids))
        .select(PageRow::as_select())
        .distinct_on(f_chapter_id)
        .order_by((f_chapter_id.asc(), f_index.asc()))
        .load(conn)
        .await
        .map_err(diesel)?;

    accept(
        rows.into_iter()
            .map(TryInto::try_into)
            .collect::<BaseRest<Vec<PageInfo>>>()?
            .into_iter()
            .map(|page_info| (page_info.chapter_id.clone(), page_info))
            .collect(),
    )
}

/// Batch-insert pages from a slice of model_entries and return the created infos.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create_batch(
    conn: &mut RdbConn,
    model_entries: &[PageEntry],
) -> BaseRest<Vec<PageInfo>> {
    //
    let entries: Vec<PageRowEntry> = model_entries
        .iter()
        .map(PageRowEntry::try_from)
        .collect::<BaseRest<_>>()?;

    let rows: Vec<PageRow> = diesel::insert_into(t_page)
        .values(&entries)
        .returning(PageRow::as_returning())
        .get_results(conn)
        .await
        .map_err(diesel)?;

    rows.into_iter().map(TryInto::try_into).collect()
}

/// Reserve a new image slot for a page: bump version, generate object key,
/// and return the reservation with previous key for cleanup.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn reserve_image(
    conn: &mut RdbConn,
    id: &str,
    file_ext: &str,
) -> BaseRest<PageImageReservation> {
    //
    let now = OffsetDateTime::now_utc();

    let (chapter_id, prev_key, raw_version): (String, Option<String>, i64) =
        t_page
            .filter(f_id.eq(id))
            .select((f_chapter_id, f_image_key, f_image_version))
            .for_update()
            .get_result(conn)
            .await
            .map_err(diesel)?;

    let image_version = next_version(raw_version)?;

    let object_key =
        PageComplex::gen_image_key(&chapter_id, id, image_version, file_ext);

    diesel::update(t_page.filter(f_id.eq(id)))
        .set((
            f_image_key.eq(Some(&object_key)),
            f_image_uploaded.eq(false),
            f_image_version.eq(i64::from(image_version)),
            f_updated_at.eq(now),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(PageImageReservation {
        object_key,
        prev_object_key: prev_key,
        image_version,
    })
}

/// Mark a page's image as successfully uploaded, checking version staleness.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn mark_image_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: u32,
    image_key: Option<&str>,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let affected = match image_key {
        //
        Some(image_key) => {
            diesel::update(
                t_page
                    .filter(f_id.eq(id))
                    .filter(f_image_version.eq(i64::from(version)))
                    .filter(f_image_key.eq(image_key)),
            )
            .set((f_image_uploaded.eq(true), f_updated_at.eq(now)))
            .execute(conn)
            .await
        }

        None => {
            diesel::update(
                t_page
                    .filter(f_id.eq(id))
                    .filter(f_image_version.eq(i64::from(version))),
            )
            .set((f_image_uploaded.eq(true), f_updated_at.eq(now)))
            .execute(conn)
            .await
        }
    }
    .map_err(diesel)?;

    if affected == 0 {
        //
        let error_message = trl("error-stale-page-image-upload");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            error_message = %error_message,
            page_id = %id,
            image_version = version,
            image_key_present = image_key.is_some(),
            image_uploaded = true,
            affected,
            stage = "mark_image_uploaded",
            "expected error: stale page image upload",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: error_message,
        });
    }

    accept(())
}

/// Sets the verified upload flag for one current page image identity.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn set_image_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: u32,
    image_key: &str,
    image_uploaded: bool,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let affected = diesel::update(
        t_page
            .filter(f_id.eq(id))
            .filter(f_image_version.eq(i64::from(version)))
            .filter(f_image_key.eq(image_key)),
    )
    .set((f_image_uploaded.eq(image_uploaded), f_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    if affected == 0 {
        //
        let error_message = trl("error-stale-page-image-upload");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            error_message = %error_message,
            page_id = %id,
            image_version = version,
            image_key_present = true,
            image_uploaded,
            affected,
            stage = "set_image_uploaded",
            "expected error: stale page image upload",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: error_message,
        });
    }

    accept(())
}

/// Persist unit counters (total, translated, proofread) onto a page row.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn set_unit_counters(
    conn: &mut RdbConn,
    id: &str,
    counters: UnitCounters,
) -> BaseRest<()> {
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

    accept(())
}

/// Delete all pages (and their child units) for a given chapter.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete_by_chapter_id(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<()> {
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

    accept(())
}

/// Deletes selected pages after deleting their child units.
#[instrument(level = "info", err(Debug), skip_all)]
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
