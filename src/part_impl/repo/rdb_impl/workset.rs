//! Diesel-backed workset repository operations.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use crate::model::read::proj::workset::WorksetInfo;
use crate::model::write::workset::{WorksetEntry, WorksetRepl};
use crate::part::repo::oper::workset::{
    AllocWorksetComicIndex, CreateWorkset, DeleteWorkset, GetWorksetInfo,
    GetWorksetInfoExcluded, ListWorksetInfos, ListWorksetInfosExcluded,
    UpdateWorkset, UpdateWorksetComicCount,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::workset::{
    WorksetAspect, WorksetRow, WorksetRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_workset::dsl::*;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::result::{diesel, expected};
use crate::shared::{RdbConn, RdbContext};

/// Workset RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

#[instrument(level = "info", err(Debug), skip_all)]
// Remove one workset row by id.
async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    // Hard-delete the row; no additional business side-effects in this layer.
    diesel::delete(t_workset.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

#[instrument(level = "info", err(Debug), skip_all)]
// Load one workset by id and return a rich info view.
async fn get_info(conn: &mut RdbConn, id: &str) -> BaseRest<WorksetInfo> {
    //
    // Fetch the row by primary key and map missing rows to `error-workset-not-found`.
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
// List worksets for one team with stable index ordering.
async fn list_infos(
    conn: &mut RdbConn,
    oper: &ListWorksetInfos<'_>,
) -> BaseRest<Vec<WorksetInfo>> {
    //
    // Apply pagination and team filter so consumers can page team worksets.
    let query = t_workset
        .filter(f_team_id.eq(oper.team_id))
        .select(WorksetRow::as_select())
        .order_by(f_index.asc())
        .offset(oper.offset as i64)
        .limit(oper.limit as i64)
        .into_boxed();

    let rows: Vec<WorksetRow> = query.load(conn).await.map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

#[instrument(level = "info", err(Debug), skip_all)]
// Update mutable metadata for an existing workset.
async fn update_info(conn: &mut RdbConn, update: &WorksetRepl) -> BaseRest<()> {
    //
    // Build an aspect object and persist nickname/description updates with timestamp.
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
// List worksets for team-level operations while excluding other readers.
async fn list_infos_excluded(
    conn: &mut RdbConn,
    team_id: &str,
) -> BaseRest<Vec<WorksetInfo>> {
    //
    // Lock rows selected by team to support follow-up serial updates.
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
// Load one workset with row lock for mutation flows.
async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<WorksetInfo> {
    //
    // Return `error-workset-not-found` when locked read sees no row.
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
// Insert a new workset and return its public info record.
async fn create(
    conn: &mut RdbConn,
    workset_entry: &WorksetEntry,
) -> BaseRest<WorksetInfo> {
    //
    // Convert API entry into row form and read back generated values.
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
// Allocate next comic index atomically for a workset.
async fn alloc_comic_index(conn: &mut RdbConn, id: &str) -> BaseRest<i32> {
    //
    // Increment and return previous next-index value in a single statement.
    let index: i32 = diesel::update(t_workset.filter(f_id.eq(id)))
        .set(f_comic_next_index.eq(f_comic_next_index + 1))
        .returning(f_comic_next_index - 1)
        .get_result(conn)
        .await
        .map_err(diesel)?;

    accept(index)
}

#[instrument(level = "info", err(Debug), skip_all)]
// Update comic count by a delta value.
async fn update_comic_count(
    conn: &mut RdbConn,
    id: &str,
    delta: i32,
) -> BaseRest<()> {
    //
    // Keep a monotonic counter aligned with comic membership updates.
    diesel::update(t_workset.filter(f_id.eq(id)))
        .set(f_comic_count.eq(f_comic_count + delta))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

impl Run<GetWorksetInfo<'_>> for RdbRepo {
    // Use BaseError to keep run-level errors consistent.
    type Error = BaseError;

    // Map `GetWorksetInfo` lookup to one repository query helper.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &GetWorksetInfo<'_>) -> BaseRest<WorksetInfo> {
        submit_query!(self.core, get_info, oper.id)
    }
}

impl Run<ListWorksetInfos<'_>> for RdbRepo {
    // Use BaseError to keep run-level errors consistent.
    type Error = BaseError;

    // Map list request into paged, team-scoped query helper.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListWorksetInfos<'_>,
    ) -> BaseRest<Vec<WorksetInfo>> {
        submit_query!(self.core, list_infos, oper)
    }
}

impl Run<UpdateWorkset<'_>> for RdbRepo {
    // Use BaseError to keep run-level errors consistent.
    type Error = BaseError;

    // Route update DTO directly into update helper.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &UpdateWorkset<'_>) -> BaseRest<()> {
        submit_query!(self.core, update_info, oper.update)
    }
}

impl Step<GetWorksetInfo<'_>, RdbContext> for RdbRepo {
    // Use BaseError for transactional context operations.
    type Error = BaseError;

    // Resolve one workset info inside transaction-scoped connection.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetWorksetInfo<'_>,
    ) -> BaseRest<WorksetInfo> {
        get_info(context.conn(), oper.id).await
    }
}

impl Step<ListWorksetInfos<'_>, RdbContext> for RdbRepo {
    // Use BaseError for transactional context operations.
    type Error = BaseError;

    // Resolve multiple worksets for team with pagination under transaction context.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListWorksetInfos<'_>,
    ) -> BaseRest<Vec<WorksetInfo>> {
        list_infos(context.conn(), oper).await
    }
}

impl Step<GetWorksetInfoExcluded<'_>, RdbContext> for RdbRepo {
    // Use BaseError for transactional context operations.
    type Error = BaseError;

    // Resolve locked workset row for mutation chains.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetWorksetInfoExcluded<'_>,
    ) -> BaseRest<WorksetInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl Step<ListWorksetInfosExcluded<'_>, RdbContext> for RdbRepo {
    // Use BaseError for transactional context operations.
    type Error = BaseError;

    // List rows locked by team id to coordinate dependent writes.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListWorksetInfosExcluded<'_>,
    ) -> BaseRest<Vec<WorksetInfo>> {
        list_infos_excluded(context.conn(), oper.team_id).await
    }
}

impl Step<CreateWorkset<'_>, RdbContext> for RdbRepo {
    // Use BaseError for transactional context operations.
    type Error = BaseError;

    // Create workset row and return inserted info representation.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateWorkset<'_>,
    ) -> BaseRest<WorksetInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl Step<DeleteWorkset<'_>, RdbContext> for RdbRepo {
    // Use BaseError for transactional context operations.
    type Error = BaseError;

    // Delete workset row as part of the current transaction.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteWorkset<'_>,
    ) -> BaseRest<()> {
        delete(context.conn(), oper.id).await
    }
}

impl Step<AllocWorksetComicIndex<'_>, RdbContext> for RdbRepo {
    // Use BaseError for transactional context operations.
    type Error = BaseError;

    // Allocate and return the next comic index for the workset.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &AllocWorksetComicIndex<'_>,
    ) -> BaseRest<i32> {
        alloc_comic_index(context.conn(), oper.id).await
    }
}

impl Step<UpdateWorksetComicCount<'_>, RdbContext> for RdbRepo {
    // Use BaseError for transactional context operations.
    type Error = BaseError;

    // Apply comic count delta to workset within transaction.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateWorksetComicCount<'_>,
    ) -> BaseRest<()> {
        update_comic_count(context.conn(), oper.id, oper.delta).await
    }
}
