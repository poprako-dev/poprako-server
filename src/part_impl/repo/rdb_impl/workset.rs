//! Diesel-backed workset repository operations.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use crate::model::workset::{WorksetEntry, WorksetInfo, WorksetInfoUpdate};
use crate::part::repo::oper::workset::{
    AllocWorksetComicIndex, CreateWorkset, DeleteWorkset, GetWorksetInfo,
    GetWorksetInfoExcluded, ListWorksetInfos, ListWorksetInfosExcluded,
    UpdateWorkset, UpdateWorksetComicCount,
};
use crate::part::repo::workset::WorksetRepo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::workset::{
    WorksetAspect, WorksetRow, WorksetRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_workset::dsl::*;
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{BaseError, BaseResult, accept};

#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

impl WorksetRepo<RdbContext> for RdbRepo {}

#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info(conn: &mut RdbConn, id: &str) -> BaseResult<WorksetInfo> {
    //
    let row: WorksetRow = t_workset
        .filter(f_id.eq(id))
        .select(WorksetRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-workset-not-found"))?;

    accept(row.into())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    oper: &ListWorksetInfos<'_>,
) -> BaseResult<Vec<WorksetInfo>> {
    //
    let mut query = t_workset
        .filter(f_team_id.eq(oper.team_id))
        .select(WorksetRow::as_select())
        .order_by(f_index.asc())
        .into_boxed();

    if let Some(page) = oper.page {
        query = query.offset(page.offset as i64).limit(page.limit as i64);
    }

    let rows: Vec<WorksetRow> = query.load(conn).await.map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn update_info(
    conn: &mut RdbConn,
    update: &WorksetInfoUpdate,
) -> BaseResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = WorksetAspect::new(now)
        .name(&update.name)
        .description(update.description.as_deref());

    diesel::update(t_workset.filter(f_id.eq(&update.id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn list_infos_excluded(
    conn: &mut RdbConn,
    team_id: &str,
) -> BaseResult<Vec<WorksetInfo>> {
    //
    let rows: Vec<WorksetRow> = t_workset
        .filter(f_team_id.eq(team_id))
        .select(WorksetRow::as_select())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseResult<WorksetInfo> {
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

    accept(row.into())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn create(
    conn: &mut RdbConn,
    workset_entry: &WorksetEntry,
) -> BaseResult<WorksetInfo> {
    //
    let entry = WorksetRowEntry::from(workset_entry);

    let row: WorksetRow = diesel::insert_into(t_workset)
        .values(&entry)
        .returning(WorksetRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    accept(row.into())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn delete(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    //
    diesel::delete(t_workset.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn alloc_comic_index(conn: &mut RdbConn, id: &str) -> BaseResult<i32> {
    //
    let index: i32 = diesel::update(t_workset.filter(f_id.eq(id)))
        .set(f_comic_next_index.eq(f_comic_next_index + 1))
        .returning(f_comic_next_index - 1)
        .get_result(conn)
        .await
        .map_err(diesel)?;

    accept(index)
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn update_comic_count(
    conn: &mut RdbConn,
    id: &str,
    delta: i32,
) -> BaseResult<()> {
    //
    diesel::update(t_workset.filter(f_id.eq(id)))
        .set(f_comic_count.eq(f_comic_count + delta))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

impl Run<GetWorksetInfo<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &GetWorksetInfo<'_>) -> BaseResult<WorksetInfo> {
        submit_query!(self.core, get_info, oper.id)
    }
}

impl Run<ListWorksetInfos<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListWorksetInfos<'_>,
    ) -> BaseResult<Vec<WorksetInfo>> {
        submit_query!(self.core, list_infos, oper)
    }
}

impl Run<UpdateWorkset<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &UpdateWorkset<'_>) -> BaseResult<()> {
        submit_query!(self.core, update_info, oper.update)
    }
}

impl Step<GetWorksetInfo<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetWorksetInfo<'_>,
    ) -> BaseResult<WorksetInfo> {
        get_info(context.conn(), oper.id).await
    }
}

impl Step<ListWorksetInfos<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListWorksetInfos<'_>,
    ) -> BaseResult<Vec<WorksetInfo>> {
        list_infos(context.conn(), oper).await
    }
}

impl Step<GetWorksetInfoExcluded<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetWorksetInfoExcluded<'_>,
    ) -> BaseResult<WorksetInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl Step<ListWorksetInfosExcluded<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListWorksetInfosExcluded<'_>,
    ) -> BaseResult<Vec<WorksetInfo>> {
        list_infos_excluded(context.conn(), oper.team_id).await
    }
}

impl Step<CreateWorkset<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateWorkset<'_>,
    ) -> BaseResult<WorksetInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl Step<DeleteWorkset<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteWorkset<'_>,
    ) -> BaseResult<()> {
        delete(context.conn(), oper.id).await
    }
}

impl Step<AllocWorksetComicIndex<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &AllocWorksetComicIndex<'_>,
    ) -> BaseResult<i32> {
        alloc_comic_index(context.conn(), oper.id).await
    }
}

impl Step<UpdateWorksetComicCount<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateWorksetComicCount<'_>,
    ) -> BaseResult<()> {
        update_comic_count(context.conn(), oper.id, oper.delta).await
    }
}
