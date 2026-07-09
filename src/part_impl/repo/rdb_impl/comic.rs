//! RDB-backed comic repository — free query functions and thin trait impls.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::complex::comic::ComicComplex;
use crate::model::comic::{
    ComicCoverReservation, ComicForm, ComicInfo, ComicInfoUpdate,
    ComicListKind, ComicListSpec,
};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::step::comic::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrChapterNextIndex,
    ListInfos, ListInfosExcluded, MarkCompleted, MarkCoverUploaded,
    ReserveCover, TouchLastActive, UpdateChapterCount, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::repo::rdb_impl::entity::comic::{
    ComicAspect, ComicEntry, ComicRow,
};
use crate::part_impl::repo::rdb_impl::{incl, RdbRepo, RdbRepoTransactional};
use crate::result::{RegularError, RegularResult};
use crate::value::chapter::{Stage, StageMask, StagePhase};
use crate::value::comic::ComicInclOpt;
use crate::value::index::user_index_to_stored_index;

use crate::part_impl::repo::rdb_impl::schema::t_comic::dsl::*;

impl ComicRepo<RdbContext> for RdbRepo {}

impl ComicRepoTransactional<RdbContext> for RdbRepoTransactional {}

// ── Free functions ──────────────────────────────────────────────────────────

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

fn two_step_predicate(
    started_column: &str,
    completed_column: &str,
    phase: StagePhase,
) -> String {
    match phase {
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

fn stored_index_from_numeric_fuzzy(fuzzy_title_value: &str) -> Option<i32> {
    match fuzzy_title_value.trim().parse() {
        Ok(index) => user_index_to_stored_index(index),
        Err(_) => None,
    }
}

async fn get_info_by_id(
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

async fn list_infos(
    conn: &mut RdbConn,
    spec: &ComicListSpec,
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
            Some(index) => query
                .filter(f_composed_title.ilike(pattern).or(f_index.eq(index))),
            None => query.filter(f_composed_title.ilike(pattern)),
        };
    }

    match &spec.kind {
        //
        ComicListKind::All => {}

        ComicListKind::Active { stages: _ } => {
            query = query.filter(f_is_completed.eq(false));
        }

        ComicListKind::Completed => {
            query = query.filter(f_is_completed.eq(true));
        }
    }

    if let ComicListKind::Active {
        stages: Some(stage_mask),
    } = &spec.kind
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

async fn update_info(
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

async fn mark_cover_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: i64,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let affected = diesel::update(
        t_comic
            .filter(f_id.eq(id))
            .filter(f_cover_version.eq(version)),
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

async fn create(
    conn: &mut RdbConn,
    form: &ComicForm,
) -> RegularResult<ComicInfo> {
    //
    let entry = ComicEntry::from(form);

    let row: ComicRow = diesel::insert_into(t_comic)
        .values(&entry)
        .returning(ComicRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(row.into())
}

async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[ComicInclOpt],
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

    let mut info: ComicInfo = row.into();

    incl::comic::populate_comic_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    Ok(info)
}

async fn list_infos_excluded(
    conn: &mut RdbConn,
    spec: &ComicListSpec,
) -> RegularResult<Vec<ComicInfo>> {
    //
    let infos = list_infos(conn, spec).await?;

    let ids = infos
        .iter()
        .map(|comic_info| comic_info.id.as_str())
        .collect::<Vec<_>>();

    let _: Vec<ComicRow> = t_comic
        .filter(f_id.eq_any(ids))
        .select(ComicRow::as_select())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(infos)
}

async fn reserve_cover(
    conn: &mut RdbConn,
    id: &str,
    file_ext: &str,
) -> RegularResult<ComicCoverReservation> {
    //
    let now = OffsetDateTime::now_utc();

    let (prev_key, new_version) = diesel::update(t_comic.filter(f_id.eq(id)))
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

    let object_key = ComicComplex::gen_cover_key(id, new_version, file_ext);

    diesel::update(t_comic.filter(f_id.eq(id)))
        .set((f_cover_key.eq(Some(&object_key)), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(ComicCoverReservation {
        object_key,
        prev_object_key: prev_key,
        cover_version: new_version,
    })
}

async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    //
    diesel::delete(t_comic.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn mark_completed(
    conn: &mut RdbConn,
    id: &str,
    is_completed: bool,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = ComicAspect::new(now).completed(is_completed);

    diesel::update(t_comic.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn incr_chapter_next_index(
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

async fn update_chapter_count(
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

async fn touch_last_active(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
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

// ── Non-transactional: Execute impls ────────────────────────────────

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> RegularResult<ComicInfo> {
        submit_query!(self.core, get_info_by_id, step.id, step.incl_opt)
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> RegularResult<Vec<ComicInfo>> {
        submit_query!(self.core, list_infos, step.spec)
    }
}

#[async_trait]
impl<'a> Execute<UpdateInfo<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &UpdateInfo<'a>) -> RegularResult<()> {
        submit_query!(self.core, update_info, step.update)
    }
}

#[async_trait]
impl<'a> Execute<MarkCoverUploaded<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &MarkCoverUploaded<'a>) -> RegularResult<()> {
        submit_query!(
            self.core,
            mark_cover_uploaded,
            step.id,
            step.cover_version
        )
    }
}

// ── Transactional: Advance impls ───────────────────────────────────

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Create<'a>,
    ) -> RegularResult<ComicInfo> {
        create(context.conn(), step.form).await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoById<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoById<'a>,
    ) -> RegularResult<ComicInfo> {
        get_info_by_id(context.conn(), step.id, step.incl_opt).await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoExcluded<'a>,
    ) -> RegularResult<ComicInfo> {
        get_info_excluded(context.conn(), step.id, step.incl_opt).await
    }
}

#[async_trait]
impl<'a> Advance<ListInfosExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ListInfosExcluded<'a>,
    ) -> RegularResult<Vec<ComicInfo>> {
        list_infos_excluded(context.conn(), step.spec).await
    }
}

#[async_trait]
impl<'a> Advance<ReserveCover<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ReserveCover<'a>,
    ) -> RegularResult<ComicCoverReservation> {
        reserve_cover(context.conn(), step.id, step.file_extension).await
    }
}

#[async_trait]
impl<'a> Advance<MarkCoverUploaded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &MarkCoverUploaded<'a>,
    ) -> RegularResult<()> {
        mark_cover_uploaded(context.conn(), step.id, step.cover_version).await
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Delete<'a>,
    ) -> RegularResult<()> {
        delete(context.conn(), step.id).await
    }
}

#[async_trait]
impl<'a> Advance<MarkCompleted<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &MarkCompleted<'a>,
    ) -> RegularResult<()> {
        mark_completed(context.conn(), step.id, step.is_completed).await
    }
}

#[async_trait]
impl<'a> Advance<IncrChapterNextIndex<'a>, RdbContext>
    for RdbRepoTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &IncrChapterNextIndex<'a>,
    ) -> RegularResult<i32> {
        incr_chapter_next_index(context.conn(), step.id).await
    }
}

#[async_trait]
impl<'a> Advance<UpdateChapterCount<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &UpdateChapterCount<'a>,
    ) -> RegularResult<()> {
        update_chapter_count(context.conn(), step.id, step.delta).await
    }
}

#[async_trait]
impl<'a> Advance<TouchLastActive<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &TouchLastActive<'a>,
    ) -> RegularResult<()> {
        touch_last_active(context.conn(), step.id).await
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;
