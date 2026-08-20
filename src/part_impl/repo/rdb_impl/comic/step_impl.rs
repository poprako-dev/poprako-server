use diesel::PgTextExpressionMethods as _;
use diesel::prelude::{
    BoolExpressionMethods as _, ExpressionMethods as _, OptionalExtension as _,
    QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::comic::ComicComplex;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::spec::comic::ComicListSpec;
use crate::model::write::comic::{
    ComicCoverReservation, ComicEntry, ComicRepl,
};
use crate::part_impl::repo::rdb_impl::comic::stage_filter::list_matching_stage_comic_ids;
use crate::part_impl::repo::rdb_impl::entity::comic::{
    ComicAspectRow, ComicEntryRow, ComicInfoRow,
};
use crate::part_impl::repo::rdb_impl::incl;
use crate::part_impl::repo::rdb_impl::schema::t_comic::dsl::{
    f_archived_at, f_chapter_count, f_chapter_next_index, f_composed_title,
    f_cover_extension, f_cover_hash, f_cover_key, f_cover_uploaded,
    f_cover_version, f_id, f_index, f_last_active_at, f_updated_at,
    f_workset_id, t_comic,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::RdbConn;
use crate::shared::result::{diesel, next_version};
use crate::value::comic::{ComicInclOpt, ComicStatus};
use crate::value::image::{ImageExt, ImageHash};
use crate::value::index::user_index_to_stored_index;

/// Queries a single comic row by ID and populates its includes.
#[instrument(level = "info", skip_all)]
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ComicInclOpt],
) -> BaseRest<ComicInfo> {
    //
    let row = t_comic
        .filter(f_id.eq(id))
        .select(ComicInfoRow::as_select())
        .get_result::<ComicInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            let err_message = trl("error-comic-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                comic_id = %id,
                stage = "get_info_by_id",
                "expected error: comic not found",
            );

            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            }
        })?;

    let mut info = row.try_into()?;

    incl::comic::populate_comic_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    accept(info)
}

/// Queries comic rows filtered by workset, optional fuzzy title, and stages.
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    conn: &mut RdbConn,
    spec: &ComicListSpec,
) -> BaseRest<Vec<ComicInfo>> {
    //
    let stage_comic_ids = match spec.stages {
        //
        Some(stage_mask) => {
            list_matching_stage_comic_ids(conn, stage_mask).await?
        }

        None => None,
    };

    let mut query = t_comic
        .filter(f_workset_id.eq(spec.workset_id.as_str()))
        .select(ComicInfoRow::as_select())
        .into_boxed();

    match spec.status {
        //
        Some(ComicStatus::Active) => {
            query = query.filter(f_archived_at.is_null());
        }

        Some(ComicStatus::Archived) => {
            query = query.filter(f_archived_at.is_not_null());
        }

        None => {}
    }

    if let Some(fuzzy_title) = &spec.fuzzy_title {
        //
        let pattern = format!("%{}%", fuzzy_title);

        query = match stored_index_from_numeric_fuzzy(fuzzy_title) {
            //
            Some(index) => query
                .filter(f_composed_title.ilike(pattern).or(f_index.eq(index))),

            None => query.filter(f_composed_title.ilike(pattern)),
        };
    }

    if let Some(comic_ids) = stage_comic_ids {
        query = query.filter(f_id.eq_any(comic_ids));
    }

    let rows = query
        .order_by((f_last_active_at.desc(), f_index.asc()))
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load::<ComicInfoRow>(conn)
        .await
        .map_err(diesel)?;

    let mut infos = rows
        .into_iter()
        .map(TryInto::try_into)
        .collect::<BaseRest<Vec<ComicInfo>>>()?;

    incl::comic::populate_comic_incls(conn, &mut infos, &spec.incl_opt).await?;

    accept(infos)
}

/// Updates the title, author, description, and composed title of a comic row.
#[instrument(level = "info", skip_all)]
pub async fn update_info(
    conn: &mut RdbConn,
    update: &ComicRepl,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let comic_info = get_info_by_id(conn, &update.id, &[]).await?;

    let composed_title = ComicComplex::compose_title(
        comic_info.index,
        &update.author,
        &update.title,
    );

    let aspect = ComicAspectRow::new(now)
        .title(&update.title)
        .author(&update.author)
        .description(update.description.as_deref())
        .composed_title(composed_title);

    diesel::update(t_comic.filter(f_id.eq(update.id.as_str())))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Marks a comic's cover as uploaded, checking for version match.
#[instrument(level = "info", skip_all)]
pub async fn mark_cover_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: u32,
    cover_key: Option<&str>,
    cover_uploaded: bool,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let affected = match cover_key {
        //
        Some(cover_key) => {
            //
            diesel::update(
                t_comic
                    .filter(f_id.eq(id))
                    .filter(f_cover_version.eq(i64::from(version)))
                    .filter(f_cover_key.eq(cover_key)),
            )
            .set((f_cover_uploaded.eq(cover_uploaded), f_updated_at.eq(now)))
            .execute(conn)
            .await
        }

        None => {
            //
            diesel::update(
                t_comic
                    .filter(f_id.eq(id))
                    .filter(f_cover_version.eq(i64::from(version))),
            )
            .set((f_cover_uploaded.eq(cover_uploaded), f_updated_at.eq(now)))
            .execute(conn)
            .await
        }
    }
    .map_err(diesel)?;

    if affected == 0 {
        //
        let err_message = trl("error-cover-version-mismatch");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            comic_id = %id,
            version,
            cover_key_present = cover_key.is_some(),
            cover_uploaded,
            affected,
            stage = "mark_cover_uploaded",
            "expected error: cover version mismatch",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    accept(())
}

/// Inserts a new comic row from the given entry and returns the created info.
#[instrument(level = "info", skip_all)]
pub async fn create(
    conn: &mut RdbConn,
    comic_entry: &ComicEntry,
) -> BaseRest<ComicInfo> {
    //
    let entry = ComicEntryRow::from(comic_entry);

    let row = diesel::insert_into(t_comic)
        .values(&entry)
        .returning(ComicInfoRow::as_returning())
        .get_result::<ComicInfoRow>(conn)
        .await
        .map_err(diesel)?;

    row.try_into()
}

/// Locks a single comic row by ID.
#[instrument(level = "info", skip_all)]
pub async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
    incls: &[ComicInclOpt],
) -> BaseRest<ComicInfo> {
    //
    let row = t_comic
        .filter(f_id.eq(id))
        .select(ComicInfoRow::as_select())
        .for_update()
        .get_result::<ComicInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            let err_message = trl("error-comic-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                comic_id = %id,
                stage = "get_info_excluded",
                "expected error: comic not found",
            );

            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            }
        })?;

    let mut comic_info = row.try_into()?;

    incl::comic::populate_comic_incls(
        conn,
        std::slice::from_mut(&mut comic_info),
        incls,
    )
    .await?;

    accept(comic_info)
}

/// Lists and locks the comic rows selected by a list spec.
#[instrument(level = "info", skip_all)]
pub async fn list_infos_excluded(
    conn: &mut RdbConn,
    spec: &ComicListSpec,
) -> BaseRest<Vec<ComicInfo>> {
    // Builds the paginated, locked row query for the list spec.
    macro_rules! load_rows {
        ($query:expr) => {
            $query
                .select(ComicInfoRow::as_select())
                .order_by((f_last_active_at.desc(), f_index.asc()))
                .offset(spec.offset as i64)
                .limit(spec.limit as i64)
                .for_update()
                .load::<ComicInfoRow>(conn)
                .await
                .map_err(diesel)?
        };
    }

    let stage_comic_ids = match spec.stages {
        //
        Some(stage_mask) => {
            list_matching_stage_comic_ids(conn, stage_mask).await?
        }

        None => None,
    };

    let rows = match (spec.fuzzy_title.as_deref(), stage_comic_ids) {
        //
        (None, None) => load_rows!(
            t_comic.filter(f_workset_id.eq(spec.workset_id.as_str()))
        ),

        (None, Some(comic_ids)) => load_rows!(
            t_comic
                .filter(f_workset_id.eq(spec.workset_id.as_str()))
                .filter(f_id.eq_any(comic_ids))
        ),

        (Some(fuzzy_title), stage_comic_ids) => {
            //
            let pattern = format!("%{}%", fuzzy_title);

            match (
                stored_index_from_numeric_fuzzy(fuzzy_title),
                stage_comic_ids,
            ) {
                //
                (Some(index), None) => load_rows!(
                    t_comic
                        .filter(f_workset_id.eq(spec.workset_id.as_str()),)
                        .filter(
                            f_composed_title
                                .ilike(pattern)
                                .or(f_index.eq(index)),
                        )
                ),

                (Some(index), Some(comic_ids)) => load_rows!(
                    t_comic
                        .filter(f_workset_id.eq(spec.workset_id.as_str()),)
                        .filter(
                            f_composed_title
                                .ilike(pattern)
                                .or(f_index.eq(index)),
                        )
                        .filter(f_id.eq_any(comic_ids))
                ),

                (None, None) => load_rows!(
                    t_comic
                        .filter(f_workset_id.eq(spec.workset_id.as_str()),)
                        .filter(f_composed_title.ilike(pattern))
                ),

                (None, Some(comic_ids)) => load_rows!(
                    t_comic
                        .filter(f_workset_id.eq(spec.workset_id.as_str()),)
                        .filter(f_composed_title.ilike(pattern))
                        .filter(f_id.eq_any(comic_ids))
                ),
            }
        }
    };

    let mut comic_infos = rows
        .into_iter()
        .map(TryInto::try_into)
        .collect::<BaseRest<Vec<_>>>()?;

    incl::comic::populate_comic_incls(conn, &mut comic_infos, &spec.incl_opt)
        .await?;

    accept(comic_infos)
}

/// Reserves a cover image key for a comic, incrementing the cover version.
#[instrument(level = "info", skip_all)]
pub async fn reserve_cover(
    conn: &mut RdbConn,
    id: &str,
    image_hash: &ImageHash,
    image_ext: ImageExt,
) -> BaseRest<ComicCoverReservation> {
    //
    let now = OffsetDateTime::now_utc();

    let (prev_key, uploaded, raw_version, stored_hash, stored_ext) = t_comic
        .filter(f_id.eq(id))
        .select((
            f_cover_key,
            f_cover_uploaded,
            f_cover_version,
            f_cover_hash,
            f_cover_extension,
        ))
        .for_update()
        .get_result::<(
            Option<String>,
            Option<bool>,
            Option<i64>,
            Option<Vec<u8>>,
            Option<String>,
        )>(conn)
        .await
        .map_err(diesel)?;

    let same_hash = prev_key.is_some()
        && stored_hash.as_deref() == Some(image_hash.as_bytes());

    if same_hash && stored_ext.as_deref() != Some(image_ext.suffix()) {
        //
        let err_message = trl("error-invalid-image-extension");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            comic_id = %id,
            image_version = raw_version,
            cover_key_present = prev_key.is_some(),
            stored_extension = ?stored_ext,
            requested_extension = %image_ext.suffix(),
            stage = "reserve_cover",
            "expected error: invalid image extension",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    if same_hash {
        //
        let object_key = prev_key.ok_or_else(|| BaseError::Unrecoverable {
            message: "[reserve_cover] pending cover key is missing".into(),
        })?;

        return accept(ComicCoverReservation {
            object_key,
            prev_object_key: None,
            cover_version: u32::try_from(raw_version.ok_or_else(|| {
                //
                BaseError::Unrecoverable {
                    message: "[reserve_cover] cover version is missing".into(),
                }
            })?)
            .map_err(|_| BaseError::Unrecoverable {
                message: "[reserve_cover] cover version is invalid".into(),
            })?,
            is_upload_required: !uploaded.unwrap_or(false),
        });
    }

    let cover_version = next_version(raw_version.unwrap_or(0))?;

    let object_key =
        ComicComplex::gen_cover_key(id, cover_version, image_ext.suffix());

    diesel::update(t_comic.filter(f_id.eq(id)))
        .set((
            f_cover_key.eq(Some(&object_key)),
            f_cover_uploaded.eq(false),
            f_cover_version.eq(i64::from(cover_version)),
            f_cover_hash.eq(image_hash.as_bytes().to_vec()),
            f_cover_extension.eq(image_ext.suffix()),
            f_updated_at.eq(now),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(ComicCoverReservation {
        object_key,
        prev_object_key: prev_key,
        cover_version,
        is_upload_required: true,
    })
}

/// Deletes a single comic row by ID.
#[instrument(level = "info", skip_all)]
pub async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    diesel::delete(t_comic.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Atomically increments and returns the previous `chapter_next_index` value.
#[instrument(level = "info", skip_all)]
pub async fn incr_chapter_next_index(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<i32> {
    //
    let prev = diesel::update(t_comic.filter(f_id.eq(id)))
        .set(f_chapter_next_index.eq(f_chapter_next_index + 1))
        .returning(f_chapter_next_index - 1)
        .get_result::<i32>(conn)
        .await
        .map_err(diesel)?;

    accept(prev)
}

/// Adjusts a comic's chapter count by the given delta.
#[instrument(level = "info", skip_all)]
pub async fn update_chapter_count(
    conn: &mut RdbConn,
    id: &str,
    delta: i32,
) -> BaseRest<()> {
    //
    diesel::update(t_comic.filter(f_id.eq(id)))
        .set(f_chapter_count.eq(f_chapter_count + delta))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Updates the `last_active_at` timestamp on a comic row to now.
#[instrument(level = "info", skip_all)]
pub async fn touch_last_active(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = ComicAspectRow::new(now).last_active_at(now);

    diesel::update(t_comic.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Parse a numeric fuzzy-title value into its stored comic index.
fn stored_index_from_numeric_fuzzy(fuzzy_title_value: &str) -> Option<i32> {
    //
    match fuzzy_title_value.trim().parse() {
        //
        Ok(index) => user_index_to_stored_index(index),

        Err(_) => None,
    }
}
