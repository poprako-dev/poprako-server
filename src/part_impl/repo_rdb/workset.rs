//! RDB-backed workset repository — free query functions and thin trait impls.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::model::workset::{WorksetForm, WorksetInfo};
use crate::part::repo::step::workset::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrComicNextIndex, ListInfosByTeamId,
    ListInfosByTeamIdExcluded, UpdateComicCount, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_rdb::entity::workset::{WorksetAspect, WorksetEntry, WorksetRow};
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional, schema};
use crate::part_impl::shared_rdb::RdbContext;
use crate::part_impl::shared_rdb::result::{diesel, expected};
use crate::result::{RegularError, RegularResult};

use schema::t_workset::dsl::*;

async fn get_workset_by_id(conn: &mut AsyncPgConnection, id: &str) -> RegularResult<WorksetInfo> {
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

async fn list_worksets_by_team(
    conn: &mut AsyncPgConnection,
    team_id: &str,
    offset: u64,
    limit: u64,
) -> RegularResult<Vec<WorksetInfo>> {
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

async fn update_workset(
    conn: &mut AsyncPgConnection,
    id: &str,
    name: &str,
    description: Option<&str>,
) -> RegularResult<()> {
    let now = OffsetDateTime::now_utc();

    let aspect = WorksetAspect::new(now).name(name).description(description);

    diesel::update(t_workset.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn list_worksets_by_team_excluded(
    conn: &mut AsyncPgConnection,
    team_id: &str,
) -> RegularResult<Vec<WorksetInfo>> {
    let rows: Vec<WorksetRow> = t_workset
        .filter(f_team_id.eq(team_id))
        .select(WorksetRow::as_select())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

async fn get_workset_by_id_excluded(
    conn: &mut AsyncPgConnection,
    id: &str,
) -> RegularResult<WorksetInfo> {
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

async fn delete_workset(conn: &mut AsyncPgConnection, id: &str) -> RegularResult<()> {
    diesel::delete(t_workset.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn get_workset_by_id_tx(
    conn: &mut AsyncPgConnection,
    id: &str,
) -> RegularResult<WorksetInfo> {
    get_workset_by_id(conn, id).await
}

async fn create_workset(
    conn: &mut AsyncPgConnection,
    form: &WorksetForm,
) -> RegularResult<WorksetInfo> {
    let entry = WorksetEntry::from(form);

    let row: WorksetRow = diesel::insert_into(t_workset)
        .values(&entry)
        .returning(WorksetRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(row.into())
}

async fn incr_comic_next_index(conn: &mut AsyncPgConnection, id: &str) -> RegularResult<i32> {
    let prev: i32 = diesel::update(t_workset.filter(f_id.eq(id)))
        .set(f_comic_next_index.eq(f_comic_next_index + 1))
        .returning(f_comic_next_index - 1)
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(prev)
}

async fn update_comic_count(
    conn: &mut AsyncPgConnection,
    id: &str,
    delta: i32,
) -> RegularResult<()> {
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

    async fn execute(&self, s: &GetInfoById<'a>) -> RegularResult<WorksetInfo> {
        submit_query!(self.shared, get_workset_by_id, s.id)
    }
}

#[async_trait]
impl<'a> Execute<ListInfosByTeamId<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, s: &ListInfosByTeamId<'a>) -> RegularResult<Vec<WorksetInfo>> {
        submit_query!(
            self.shared,
            list_worksets_by_team,
            s.team_id,
            s.offset,
            s.limit
        )
    }
}

#[async_trait]
impl<'a> Execute<UpdateInfo<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, s: &UpdateInfo<'a>) -> RegularResult<()> {
        submit_query!(
            self.shared,
            update_workset,
            s.update.id.as_str(),
            &s.update.name,
            s.update.description.as_deref()
        )
    }
}

// ── Transactional: Advance impls ───────────────────────────────────

#[async_trait]
impl<'a> Advance<ListInfosByTeamIdExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        c: &mut RdbContext,
        s: &ListInfosByTeamIdExcluded<'a>,
    ) -> RegularResult<Vec<WorksetInfo>> {
        list_worksets_by_team_excluded(c.conn(), s.team_id).await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        c: &mut RdbContext,
        s: &GetInfoExcluded<'a>,
    ) -> RegularResult<WorksetInfo> {
        get_workset_by_id_excluded(c.conn(), s.id).await
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(&self, c: &mut RdbContext, s: &Delete<'a>) -> RegularResult<()> {
        delete_workset(c.conn(), s.id).await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoById<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(&self, c: &mut RdbContext, s: &GetInfoById<'a>) -> RegularResult<WorksetInfo> {
        get_workset_by_id_tx(c.conn(), s.id).await
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(&self, c: &mut RdbContext, s: &Create<'a>) -> RegularResult<WorksetInfo> {
        create_workset(c.conn(), s.form).await
    }
}

#[async_trait]
impl<'a> Advance<IncrComicNextIndex<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(&self, c: &mut RdbContext, s: &IncrComicNextIndex<'a>) -> RegularResult<i32> {
        incr_comic_next_index(c.conn(), s.id).await
    }
}

#[async_trait]
impl<'a> Advance<UpdateComicCount<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(&self, c: &mut RdbContext, s: &UpdateComicCount<'a>) -> RegularResult<()> {
        update_comic_count(c.conn(), s.id, s.delta).await
    }
}
