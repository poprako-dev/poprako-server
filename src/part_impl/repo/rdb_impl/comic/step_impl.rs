use diesel::PgTextExpressionMethods as _;
use diesel::prelude::{
    BoolExpressionMethods as _, ExpressionMethods as _, OptionalExtension as _,
    QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::complex::comic::ComicComplex;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::spec::comic::ComicListSpec;
use crate::model::write::comic::{ComicEntry, ComicRepl};
use crate::part_impl::repo::rdb_impl::comic::stage_filter::list_matching_stage_comic_ids;
use crate::part_impl::repo::rdb_impl::entity::comic::{
    ComicAspectRow, ComicEntryRow, ComicInfoRow,
};
use crate::part_impl::repo::rdb_impl::incl;
use crate::part_impl::repo::rdb_impl::numeric::usize_from_i32;
use crate::part_impl::repo::rdb_impl::schema::t_comic::dsl::{
    f_archived_at, f_chapter_count, f_chapter_next_index, f_composed_title,
    f_deleted_at, f_id, f_index, f_last_active_at, f_workset_id, t_comic,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;
use crate::value::comic::{ComicInclOpt, ComicStatus};
use crate::value::index::user_index_to_stored_index;

/// Build the expected error for a missing comic.
pub fn missing_comic(id: &str, operation: &str) -> BaseError {
    //
    let err_message = trl("error-comic-not-found");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        comic_id = %id,
        operation,
        "expected error: comic not found",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
}

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
        .filter(f_deleted_at.is_null())
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
        .filter(f_deleted_at.is_null())
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
        let pattern = format!("%{}%", escape_ilike_pattern(fuzzy_title));

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
        .offset(i64::from(spec.offset))
        .limit(i64::from(spec.limit))
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

    let updated_count = diesel::update(
        t_comic
            .filter(f_id.eq(update.id.as_str()))
            .filter(f_deleted_at.is_null()),
    )
    .set(&aspect)
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_count == 0 {
        return Err(missing_comic(&update.id, "update comic info"));
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
    let entry = ComicEntryRow::try_from(comic_entry)?;

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
        .filter(f_deleted_at.is_null())
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

/// Atomically increments and returns the previous `chapter_next_index` value.
#[instrument(level = "info", skip_all)]
pub async fn incr_chapter_next_index(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<usize> {
    //
    let prev = diesel::update(
        t_comic.filter(f_id.eq(id)).filter(f_deleted_at.is_null()),
    )
    .set(f_chapter_next_index.eq(f_chapter_next_index + 1))
    .returning(f_chapter_next_index - 1)
    .get_result::<i32>(conn)
    .await
    .optional()
    .map_err(diesel)?;

    let Some(prev) = prev else {
        return Err(missing_comic(id, "allocate comic chapter index"));
    };

    accept(usize_from_i32(prev, "t_comic.f_chapter_next_index")?)
}

/// Adjusts a comic's chapter count by the given delta.
#[instrument(level = "info", skip_all)]
pub async fn update_chapter_count(
    conn: &mut RdbConn,
    id: &str,
    delta: i32,
) -> BaseRest<()> {
    //
    let updated_count = diesel::update(
        t_comic.filter(f_id.eq(id)).filter(f_deleted_at.is_null()),
    )
    .set(f_chapter_count.eq(f_chapter_count + delta))
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_count == 0 {
        return Err(missing_comic(id, "update comic chapter count"));
    }

    accept(())
}

/// Updates the `last_active_at` timestamp on a comic row to now.
#[instrument(level = "info", skip_all)]
pub async fn touch_last_active(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = ComicAspectRow::new(now).last_active_at(now);

    let updated_count = diesel::update(
        t_comic.filter(f_id.eq(id)).filter(f_deleted_at.is_null()),
    )
    .set(&aspect)
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_count == 0 {
        return Err(missing_comic(id, "touch comic last active"));
    }

    accept(())
}

// Parse a numeric fuzzy-title value into its stored comic index.
fn stored_index_from_numeric_fuzzy(fuzzy_title_value: &str) -> Option<i32> {
    //
    fuzzy_title_value
        .trim()
        .parse::<usize>()
        .ok()
        .and_then(user_index_to_stored_index)
        .and_then(|index| i32::try_from(index).ok())
}

// Escape wildcard characters for a PostgreSQL ILIKE pattern.
fn escape_ilike_pattern(input: &str) -> String {
    //
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
