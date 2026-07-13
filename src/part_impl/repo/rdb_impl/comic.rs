use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use crate::complex::comic::ComicComplex;
use crate::model::comic::{
    ComicCoverReservation, ComicEntry, ComicInfo, ComicInfoListKind,
    ComicInfoListSpec, ComicInfoUpdate,
};
use crate::part::repo::comic::ComicRepo;
use crate::part_impl::repo::rdb_impl::entity::comic::{
    ComicAspect, ComicRow, ComicRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_comic::dsl::*;
use crate::part_impl::repo::rdb_impl::{RdbRepo, incl};
use crate::part_impl::shared::result::{diesel, expected, version};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::RegularResult;
use crate::value::chapter::{Stage, StageMask, StagePhase};
use crate::value::comic::ComicInclOpt;
use crate::value::index::user_index_to_stored_index;

impl ComicRepo<RdbContext> for RdbRepo {}

mod orchestra;

/// Generates a raw SQL predicate for a single-stage (one-shot) workflow column and phase.
fn one_shot_predicate(column: &str, phase: StagePhase) -> &'static str {
    match (column, phase) {
        //
        ("f_uploaded_at", StagePhase::Pending) => {
            "pinned_chapter.f_uploaded_at IS NULL"
        }

        ("f_uploaded_at", StagePhase::Completed) => {
            "pinned_chapter.f_uploaded_at IS NOT NULL"
        }

        ("f_reviewed_at", StagePhase::Pending) => {
            "pinned_chapter.f_reviewed_at IS NULL"
        }

        ("f_reviewed_at", StagePhase::Completed) => {
            "pinned_chapter.f_reviewed_at IS NOT NULL"
        }

        ("f_published_at", StagePhase::Pending) => {
            "pinned_chapter.f_published_at IS NULL"
        }

        ("f_published_at", StagePhase::Completed) => {
            "pinned_chapter.f_published_at IS NOT NULL"
        }

        (_, StagePhase::Active) => "FALSE",

        _ => "FALSE",
    }
}

/// Generates a SQL predicate for a two-stage (started/completed) workflow column and phase.
fn two_step_predicate(
    started_column: &str,
    completed_column: &str,
    phase: StagePhase,
) -> String {
    match phase {
        //
        StagePhase::Pending => format!(
            "pinned_chapter.{} IS NULL AND pinned_chapter.{} IS NULL",
            started_column, completed_column,
        ),

        StagePhase::Active => format!(
            "pinned_chapter.{} IS NOT NULL AND pinned_chapter.{} IS NULL",
            started_column, completed_column,
        ),

        StagePhase::Completed => {
            format!("pinned_chapter.{} IS NOT NULL", completed_column)
        }
    }
}

/// Generates a workflow predicate for a given stage and phase.
fn stage_predicate(stage: Stage, phase: StagePhase) -> String {
    match stage {
        //
        Stage::RawProvide => one_shot_predicate("f_uploaded_at", phase).into(),

        Stage::Translate => {
            two_step_predicate("f_translating_at", "f_translated_at", phase)
        }

        Stage::Proofread => {
            two_step_predicate("f_proofreading_at", "f_proofread_at", phase)
        }

        Stage::TypesetRedraw => {
            two_step_predicate("f_typesetting_at", "f_typeset_at", phase)
        }

        Stage::Review => one_shot_predicate("f_reviewed_at", phase).into(),

        Stage::Publish => one_shot_predicate("f_published_at", phase).into(),
    }
}

/// Builds an optional `EXISTS` subquery SQL string from a stage mask workflow filter.
fn workflow_filter_sql(stage_mask: StageMask) -> Option<String> {
    //
    let predicates = StageMask::stages()
        .iter()
        .filter(|stage| !stage_mask.ignores_stage(**stage))
        .map(|stage| stage_predicate(*stage, stage_mask.get_phase(*stage)))
        .collect::<Vec<_>>();

    if predicates.is_empty() {
        return None;
    }

    let mut sql = String::from(
        "EXISTS (SELECT 1 FROM t_chapter AS pinned_chapter \
         WHERE pinned_chapter.f_comic_id = t_comic.f_id \
         AND pinned_chapter.f_is_pinned = TRUE",
    );

    for predicate in predicates {
        //
        sql.push_str(" AND ");

        sql.push_str(&predicate);
    }

    sql.push(')');

    Some(sql)
}

/// Parses a fuzzy title value as an integer and converts to a stored index.
fn stored_index_from_numeric_fuzzy(fuzzy_title_value: &str) -> Option<i32> {
    match fuzzy_title_value.trim().parse() {
        //
        Ok(index) => user_index_to_stored_index(index),

        Err(_) => None,
    }
}

/// Queries a single comic row by ID and populates its includes.
pub(super) async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ComicInclOpt],
) -> RegularResult<ComicInfo> {
    //
    let row: ComicRow = t_comic
        .filter(f_id.eq(id))
        .select(ComicRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-comic-not-found"))?;

    let mut info: ComicInfo = row.into();

    incl::comic::populate_comic_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    Ok(info)
}

/// Queries comic rows filtered by workset, optional fuzzy title, and list kind.
pub(super) async fn list_infos(
    conn: &mut RdbConn,
    spec: &ComicInfoListSpec,
) -> RegularResult<Vec<ComicInfo>> {
    //
    let mut query = t_comic
        .filter(f_workset_id.eq(spec.workset_id.as_str()))
        .select(ComicRow::as_select())
        .into_boxed();

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

    if let ComicInfoListKind::Stages(stage_mask) = &spec.kind
        && let Some(sql) = workflow_filter_sql(*stage_mask)
    {
        query = query.filter(diesel::dsl::sql::<Bool>(&sql));
    }

    let rows: Vec<ComicRow> = query
        .order_by((f_last_active_at.desc(), f_index.asc()))
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    let mut infos: Vec<ComicInfo> = rows.into_iter().map(Into::into).collect();

    incl::comic::populate_comic_incls(conn, &mut infos, &spec.incl_opt).await?;

    Ok(infos)
}

/// Updates the title, author, description, and composed title of a comic row.
pub(super) async fn update_info(
    conn: &mut RdbConn,
    update: &ComicInfoUpdate,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let comic_info = get_info_by_id(conn, &update.id, &[]).await?;

    let composed_title = ComicComplex::compose_title(
        comic_info.index,
        &update.author,
        &update.title,
    );

    let aspect = ComicAspect::new(now)
        .title(&update.title)
        .author(&update.author)
        .description(update.description.as_deref())
        .composed_title(composed_title);

    diesel::update(t_comic.filter(f_id.eq(update.id.as_str())))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Marks a comic's cover as uploaded, checking for version match.
pub(super) async fn mark_cover_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: u32,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let affected = diesel::update(
        t_comic
            .filter(f_id.eq(id))
            .filter(f_cover_version.eq(i64::from(version))),
    )
    .set((f_cover_uploaded.eq(true), f_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    if affected == 0 {
        return Err(expected("error-cover-version-mismatch"));
    }

    Ok(())
}

/// Inserts a new comic row from the given entry and returns the created info.
pub(super) async fn create(
    conn: &mut RdbConn,
    comic_entry: &ComicEntry,
) -> RegularResult<ComicInfo> {
    //
    let entry = ComicRowEntry::from(comic_entry);

    let row: ComicRow = diesel::insert_into(t_comic)
        .values(&entry)
        .returning(ComicRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(row.into())
}

/// Locks a single comic row by ID.
pub(super) async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
    incls: &[ComicInclOpt],
) -> RegularResult<ComicInfo> {
    //
    let row: ComicRow = t_comic
        .filter(f_id.eq(id))
        .select(ComicRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-comic-not-found"))?;

    let mut comic_info = row.into();

    incl::comic::populate_comic_incls(
        conn,
        std::slice::from_mut(&mut comic_info),
        incls,
    )
    .await?;

    Ok(comic_info)
}

/// Lists and locks the comic rows selected by a list spec.
pub(super) async fn list_infos_excluded(
    conn: &mut RdbConn,
    spec: &ComicInfoListSpec,
) -> RegularResult<Vec<ComicInfo>> {
    //
    let mut predicate: Box<
        dyn BoxableExpression<t_comic, Pg, SqlType = Bool> + '_,
    > = Box::new(f_workset_id.eq(spec.workset_id.as_str()));

    if let Some(fuzzy_title) = &spec.fuzzy_title {
        //
        let pattern = format!("%{}%", fuzzy_title);

        predicate = match stored_index_from_numeric_fuzzy(fuzzy_title) {
            //
            Some(index) => Box::new(
                predicate
                    .and(f_composed_title.ilike(pattern).or(f_index.eq(index))),
            ),

            None => Box::new(predicate.and(f_composed_title.ilike(pattern))),
        };
    }

    if let ComicInfoListKind::Stages(stage_mask) = &spec.kind
        && let Some(sql) = workflow_filter_sql(*stage_mask)
    {
        predicate = Box::new(predicate.and(diesel::dsl::sql::<Bool>(&sql)));
    }

    let rows: Vec<ComicRow> = t_comic
        .filter(predicate)
        .select(ComicRow::as_select())
        .order_by((f_last_active_at.desc(), f_index.asc()))
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    let mut comic_infos = rows.into_iter().map(Into::into).collect::<Vec<_>>();

    incl::comic::populate_comic_incls(conn, &mut comic_infos, &spec.incl_opt)
        .await?;

    Ok(comic_infos)
}

/// Reserves a cover image key for a comic, incrementing the cover version.
pub(super) async fn reserve_cover(
    conn: &mut RdbConn,
    id: &str,
    file_ext: &str,
) -> RegularResult<ComicCoverReservation> {
    //
    let now = OffsetDateTime::now_utc();

    let (prev_key, raw_version): (Option<String>, i64) =
        diesel::update(t_comic.filter(f_id.eq(id)))
            .set((
                f_cover_key.eq::<Option<&str>>(None),
                f_cover_uploaded.eq(false),
                f_cover_version.eq(f_cover_version + 1),
                f_updated_at.eq(now),
            ))
            .returning((f_cover_key, f_cover_version))
            .get_result(conn)
            .await
            .map_err(diesel)?;

    let cover_version = version(raw_version)?;

    let object_key = ComicComplex::gen_cover_key(id, cover_version, file_ext);

    diesel::update(t_comic.filter(f_id.eq(id)))
        .set((f_cover_key.eq(Some(&object_key)), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(ComicCoverReservation {
        object_key,
        prev_object_key: prev_key,
        cover_version,
    })
}

/// Deletes a single comic row by ID.
pub(super) async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    //
    diesel::delete(t_comic.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Atomically increments and returns the previous `chapter_next_index` value.
pub(super) async fn incr_chapter_next_index(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<i32> {
    //
    let prev: i32 = diesel::update(t_comic.filter(f_id.eq(id)))
        .set(f_chapter_next_index.eq(f_chapter_next_index + 1))
        .returning(f_chapter_next_index - 1)
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(prev)
}

/// Adjusts a comic's chapter count by the given delta.
pub(super) async fn update_chapter_count(
    conn: &mut RdbConn,
    id: &str,
    delta: i32,
) -> RegularResult<()> {
    //
    diesel::update(t_comic.filter(f_id.eq(id)))
        .set(f_chapter_count.eq(f_chapter_count + delta))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Updates the `last_active_at` timestamp on a comic row to now.
pub(super) async fn touch_last_active(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = ComicAspect::new(now).last_active_at(now);

    diesel::update(t_comic.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

#[cfg(all(test, feature = "repo"))]
mod tests;
