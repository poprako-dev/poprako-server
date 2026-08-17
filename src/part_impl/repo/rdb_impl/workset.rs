//! Diesel-backed workset repository operations.

/// Workset RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{AtLeast, Level, Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::read::proj::workset::WorksetInfo;
use crate::model::write::workset::{WorksetEntry, WorksetRepl};
use crate::part::repo::oper::workset::{
    AllocWorksetComicIndex, CreateWorkset, DeleteWorkset, GetWorksetInfo,
    GetWorksetInfoExcluded, ListWorksetInfos, ListWorksetInfosExcluded,
    UpdateWorkset, UpdateWorksetComicCount,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::workset::{
    WorksetAspectRow, WorksetEntryRow, WorksetInfoRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_workset::dsl::*;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;
use crate::shared::{RdbConn, RdbContext};

#[instrument(level = "info", skip_all)]
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

#[instrument(level = "info", skip_all)]
// Load one workset by id and return a rich info view.
async fn get_info(conn: &mut RdbConn, id: &str) -> BaseRest<WorksetInfo> {
    //
    // Fetch the row by primary key and map missing rows to `error-workset-not-found`.
    let row = t_workset
        .filter(f_id.eq(id))
        .select(WorksetInfoRow::as_select())
        .get_result::<WorksetInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        //
        let message = trl("error-workset-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            workset_id = %id,
            operation = "get workset info",
            "expected workset error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    };

    accept(row.into())
}

#[instrument(level = "info", skip_all)]
// List worksets for one team with stable index ordering.
async fn list_infos(
    conn: &mut RdbConn,
    oper: &ListWorksetInfos<'_>,
) -> BaseRest<Vec<WorksetInfo>> {
    //
    // Apply pagination and team filter so consumers can page team worksets.
    let query = t_workset
        .filter(f_team_id.eq(oper.team_id))
        .select(WorksetInfoRow::as_select())
        .order_by(f_index.asc())
        .offset(oper.offset as i64)
        .limit(oper.limit as i64)
        .into_boxed();

    let rows = query.load::<WorksetInfoRow>(conn).await.map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

#[instrument(level = "info", skip_all)]
// Update mutable metadata for an existing workset.
async fn update_info(conn: &mut RdbConn, update: &WorksetRepl) -> BaseRest<()> {
    //
    // Build an aspect object and persist nickname/description updates with timestamp.
    let now = OffsetDateTime::now_utc();

    let aspect = WorksetAspectRow::new(now)
        .name(&update.name)
        .description(update.description.as_deref());

    diesel::update(t_workset.filter(f_id.eq(&update.id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

#[instrument(level = "info", skip_all)]
// List worksets for team-level operations while excluding other readers.
async fn list_infos_excluded(
    conn: &mut RdbConn,
    team_id: &str,
) -> BaseRest<Vec<WorksetInfo>> {
    //
    // Lock rows selected by team to support follow-up serial updates.
    let rows = t_workset
        .filter(f_team_id.eq(team_id))
        .select(WorksetInfoRow::as_select())
        .for_update()
        .load::<WorksetInfoRow>(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

#[instrument(level = "info", skip_all)]
// Load one workset with row lock for mutation flows.
async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<WorksetInfo> {
    //
    // Return `error-workset-not-found` when locked read sees no row.
    let row = t_workset
        .filter(f_id.eq(id))
        .select(WorksetInfoRow::as_select())
        .for_update()
        .get_result::<WorksetInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        //
        let message = trl("error-workset-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            workset_id = %id,
            operation = "lock workset info",
            "expected workset error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    };

    accept(row.into())
}

#[instrument(level = "info", skip_all)]
// Insert a new workset and return its public info record.
async fn create(
    conn: &mut RdbConn,
    workset_entry: &WorksetEntry,
) -> BaseRest<WorksetInfo> {
    //
    // Convert API entry into row form and read back generated values.
    let entry = WorksetEntryRow::from(workset_entry);

    let row = diesel::insert_into(t_workset)
        .values(&entry)
        .returning(WorksetInfoRow::as_returning())
        .get_result::<WorksetInfoRow>(conn)
        .await
        .map_err(diesel)?;

    accept(row.into())
}

#[instrument(level = "info", skip_all)]
// Allocate next comic index atomically for a workset.
async fn alloc_comic_index(conn: &mut RdbConn, id: &str) -> BaseRest<i32> {
    //
    // Increment and return previous next-index value in a single statement.
    let index = diesel::update(t_workset.filter(f_id.eq(id)))
        .set(f_comic_next_index.eq(f_comic_next_index + 1))
        .returning(f_comic_next_index - 1)
        .get_result::<i32>(conn)
        .await
        .map_err(diesel)?;

    accept(index)
}

#[instrument(level = "info", skip_all)]
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

impl Run<GetWorksetInfo<'_>> for HybRepo {
    // Use BaseError to keep run-level errors consistent.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Map `GetWorksetInfo` lookup to one repository query helper.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &GetWorksetInfo<'_>) -> BaseRest<WorksetInfo> {
        submit_query!(self.core, get_info, oper.id)
    }
}

impl Run<ListWorksetInfos<'_>> for HybRepo {
    // Use BaseError to keep run-level errors consistent.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Map list request into paged, team-scoped query helper.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ListWorksetInfos<'_>,
    ) -> BaseRest<Vec<WorksetInfo>> {
        submit_query!(self.core, list_infos, oper)
    }
}

impl Run<UpdateWorkset<'_>> for HybRepo {
    // Use BaseError to keep run-level errors consistent.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Route update DTO directly into update helper.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &UpdateWorkset<'_>) -> BaseRest<()> {
        submit_query!(self.core, update_info, oper.update)
    }
}

impl<L> Step<GetWorksetInfo<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send,
    L: AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Use BaseError for transactional context operations.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve one workset info inside transaction-scoped connection.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetWorksetInfo<'_>,
    ) -> BaseRest<WorksetInfo> {
        get_info(context.conn(), oper.id).await
    }
}

impl<L> Step<ListWorksetInfos<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send,
    L: AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Use BaseError for transactional context operations.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve multiple worksets for team with pagination under transaction context.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListWorksetInfos<'_>,
    ) -> BaseRest<Vec<WorksetInfo>> {
        list_infos(context.conn(), oper).await
    }
}

impl<L> Step<GetWorksetInfoExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send,
    L: AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Use BaseError for transactional context operations.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve locked workset row for mutation chains.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetWorksetInfoExcluded<'_>,
    ) -> BaseRest<WorksetInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl<L> Step<ListWorksetInfosExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send,
    L: AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Use BaseError for transactional context operations.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // List rows locked by team id to coordinate dependent writes.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListWorksetInfosExcluded<'_>,
    ) -> BaseRest<Vec<WorksetInfo>> {
        list_infos_excluded(context.conn(), oper.team_id).await
    }
}

impl<L> Step<CreateWorkset<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send,
    L: AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Use BaseError for transactional context operations.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Create workset row and return inserted info representation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateWorkset<'_>,
    ) -> BaseRest<WorksetInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<L> Step<DeleteWorkset<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send,
    L: AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Use BaseError for transactional context operations.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Delete workset row as part of the current transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeleteWorkset<'_>,
    ) -> BaseRest<()> {
        delete(context.conn(), oper.id).await
    }
}

impl<L> Step<AllocWorksetComicIndex<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send,
    L: AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Use BaseError for transactional context operations.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Allocate and return the next comic index for the workset.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &AllocWorksetComicIndex<'_>,
    ) -> BaseRest<i32> {
        alloc_comic_index(context.conn(), oper.id).await
    }
}

impl<L> Step<UpdateWorksetComicCount<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send,
    L: AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Use BaseError for transactional context operations.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Apply comic count delta to workset within transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdateWorksetComicCount<'_>,
    ) -> BaseRest<()> {
        update_comic_count(context.conn(), oper.id, oper.delta).await
    }
}
