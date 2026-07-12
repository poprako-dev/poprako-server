//! RDB-backed workset repository — free query functions and thin trait impls.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::model::workset_model;
use crate::part::repo::step::workset::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrComicNextIndex,
    ListAllInfosByTeamIdExcluded, ListInfosByTeamId, UpdateComicCount,
    UpdateInfo,
};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::rdb_impl::entity::workset::{
    WorksetAspect, WorksetEntry, WorksetRow,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, RdbRepoTransactional};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{RegularError, RegularResult};

use crate::part_impl::repo::rdb_impl::schema::t_workset::dsl::*;

impl WorksetRepo<RdbContext> for RdbRepo {}

impl WorksetRepoTransactional<RdbContext> for RdbRepoTransactional {}

/// Load a single workset info by ID.
async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<workset_model::Info> {
    //
    let row: WorksetRow = t_workset
        .filter(f_id.eq(id))
        .select(WorksetRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-workset-not-found"))?;

    Ok(row.into())
}

/// Query a paginated list of worksets for a team, ordered by index.
async fn list_infos_by_team_id(
    conn: &mut RdbConn,
    team_id: &str,
    offset: u64,
    limit: u64,
) -> RegularResult<Vec<workset_model::Info>> {
    //
    let rows: Vec<WorksetRow> = t_workset
        .filter(f_team_id.eq(team_id))
        .select(WorksetRow::as_select())
        .order_by(f_index.asc())
        .offset(offset as i64)
        .limit(limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Update a workset's name and optional description.
async fn update_info(
    conn: &mut RdbConn,
    id: &str,
    name: &str,
    description: Option<&str>,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = WorksetAspect::new(now).name(name).description(description);

    diesel::update(t_workset.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Query all worksets for a team, locking the rows for update.
async fn list_all_infos_by_team_id_excluded(
    conn: &mut RdbConn,
    team_id: &str,
) -> RegularResult<Vec<workset_model::Info>> {
    //
    let rows: Vec<WorksetRow> = t_workset
        .filter(f_team_id.eq(team_id))
        .select(WorksetRow::as_select())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Load a workset info by ID, locking the row for update.
async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<workset_model::Info> {
    //
    let row: WorksetRow = t_workset
        .filter(f_id.eq(id))
        .select(WorksetRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-workset-not-found"))?;

    Ok(row.into())
}

/// Delete a workset by ID.
async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    //
    diesel::delete(t_workset.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Insert a new workset and return its info.
async fn create(
    conn: &mut RdbConn,
    form: &workset_model::Form,
) -> RegularResult<workset_model::Info> {
    //
    let entry = WorksetEntry::from(form);

    let row: WorksetRow = diesel::insert_into(t_workset)
        .values(&entry)
        .returning(WorksetRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(row.into())
}

/// Atomically increment and return the previous comic-next-index for a workset.
async fn incr_comic_next_index(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<i32> {
    //
    let prev: i32 = diesel::update(t_workset.filter(f_id.eq(id)))
        .set(f_comic_next_index.eq(f_comic_next_index + 1))
        .returning(f_comic_next_index - 1)
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(prev)
}

/// Adjust a workset's comic count by a delta (positive or negative).
async fn update_comic_count(
    conn: &mut RdbConn,
    id: &str,
    delta: i32,
) -> RegularResult<()> {
    //
    diesel::update(t_workset.filter(f_id.eq(id)))
        .set(f_comic_count.eq(f_comic_count + delta))
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
    ) -> RegularResult<workset_model::Info> {
        submit_query!(self.core, get_info_by_id, step.id)
    }
}

#[async_trait]
impl<'a> Execute<ListInfosByTeamId<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfosByTeamId<'a>,
    ) -> RegularResult<Vec<workset_model::Info>> {
        submit_query!(
            self.core,
            list_infos_by_team_id,
            step.team_id,
            step.offset,
            step.limit
        )
    }
}

#[async_trait]
impl<'a> Execute<UpdateInfo<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &UpdateInfo<'a>) -> RegularResult<()> {
        submit_query!(
            self.core,
            update_info,
            step.update.id.as_str(),
            &step.update.name,
            step.update.description.as_deref()
        )
    }
}

// ── Transactional: Advance impls ───────────────────────────────────

#[async_trait]
impl<'a> Advance<ListAllInfosByTeamIdExcluded<'a>, RdbContext>
    for RdbRepoTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ListAllInfosByTeamIdExcluded<'a>,
    ) -> RegularResult<Vec<workset_model::Info>> {
        list_all_infos_by_team_id_excluded(context.conn(), step.team_id).await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoExcluded<'a>,
    ) -> RegularResult<workset_model::Info> {
        get_info_excluded(context.conn(), step.id).await
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
impl<'a> Advance<GetInfoById<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoById<'a>,
    ) -> RegularResult<workset_model::Info> {
        get_info_by_id(context.conn(), step.id).await
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Create<'a>,
    ) -> RegularResult<workset_model::Info> {
        create(context.conn(), step.form).await
    }
}

#[async_trait]
impl<'a> Advance<IncrComicNextIndex<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &IncrComicNextIndex<'a>,
    ) -> RegularResult<i32> {
        incr_comic_next_index(context.conn(), step.id).await
    }
}

#[async_trait]
impl<'a> Advance<UpdateComicCount<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &UpdateComicCount<'a>,
    ) -> RegularResult<()> {
        update_comic_count(context.conn(), step.id, step.delta).await
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;
