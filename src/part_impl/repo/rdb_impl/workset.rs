//! Diesel-backed workset repository operations.

/// Workset RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::workset::WorksetInfo;
use crate::model::write::workset::{WorksetEntry, WorksetRepl};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::workset::{
    AllocWorksetComicIndex, CreateWorkset, GetWorksetInfo, ListWorksetInfos,
    UpdateWorkset, UpdateWorksetComicCount,
};
use crate::part_impl::repo::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::workset::{
    WorksetInfoRecord, WorksetItem, WorksetPack,
};
use crate::part_impl::repo::rdb_impl::numeric::usize_from_i32;
use crate::part_impl::repo::rdb_impl::schema::t_workset::dsl::{
    f_comic_count, f_comic_next_index, f_deleted_at, f_id, f_index, f_team_id,
    t_workset,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::shared::RdbContext;
use crate::shared::result::diesel;

/// Build the expected error for a missing workset.
pub fn missing_workset(id: &str, operation: &str) -> BaseError {
    //
    expected_error! {
        ExpectedVariant::Args,
        message: trl("error-workset-not-found");
        workset_id = %id,
        operation,
        "workset operation rejected",
    }
}

// Load one workset by id and return a rich info view.
#[instrument(level = "info", skip_all)]
async fn get_info(conn: &mut RdbConn, id: &str) -> BaseRest<WorksetInfo> {
    //
    // Fetch the record by primary key and map missing records to `error-workset-not-found`.
    let record = t_workset
        .filter(f_id.eq(id))
        .filter(f_deleted_at.is_null())
        .select(WorksetInfoRecord::as_select())
        .get_result::<WorksetInfoRecord>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(record) = record else {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-workset-not-found");
            error_variant = ?ExpectedVariant::Args,
            workset_id = %id,
            operation = "get workset info",
            "workset operation rejected",
        });
    };

    accept!(WorksetInfo::try_from(record)?)
}

// List worksets for one team with stable index ordering.
#[instrument(level = "info", skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    oper: &ListWorksetInfos<'_>,
) -> BaseRest<Vec<WorksetInfo>> {
    //
    // Apply pagination and team filter so consumers can page team worksets.
    let query = t_workset
        .filter(f_team_id.eq(oper.team_id))
        .filter(f_deleted_at.is_null())
        .select(WorksetInfoRecord::as_select())
        .order_by(f_index.asc())
        .offset(i64::from(oper.offset))
        .limit(i64::from(oper.limit.get()))
        .into_boxed();

    let records = query
        .load::<WorksetInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    records.into_iter().map(WorksetInfo::try_from).collect()
}

// Update mutable metadata for an existing workset.
#[instrument(level = "info", skip_all)]
async fn update_info(conn: &mut RdbConn, update: &WorksetRepl) -> BaseRest<()> {
    //
    // Build a pack and persist nickname/description updates with timestamp.
    let now = OffsetDateTime::now_utc();

    let pack = WorksetPack::new(now)
        .name(update.trimmed_name())
        .description(update.trimmed_description());

    let updated_count = diesel::update(
        t_workset
            .filter(f_id.eq(&update.id))
            .filter(f_deleted_at.is_null()),
    )
    .set(&pack)
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_count == 0 {
        return Err(missing_workset(&update.id, "update workset info"));
    }

    accept!(())
}

// Insert a new workset and return its public info record.
#[instrument(level = "info", skip_all)]
async fn create(
    conn: &mut RdbConn,
    entry: &WorksetEntry,
) -> BaseRest<WorksetInfo> {
    //
    // Convert the API entry into a database item and read back generated values.
    let item = WorksetItem::try_from(entry)?;

    let record = diesel::insert_into(t_workset)
        .values(&item)
        .returning(WorksetInfoRecord::as_returning())
        .get_result::<WorksetInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    accept!(WorksetInfo::try_from(record)?)
}

// Allocate next comic index atomically for a workset.
#[instrument(level = "info", skip_all)]
async fn alloc_comic_index(conn: &mut RdbConn, id: &str) -> BaseRest<usize> {
    //
    // Increment and return previous next-index value in a single statement.
    let index = diesel::update(
        t_workset.filter(f_id.eq(id)).filter(f_deleted_at.is_null()),
    )
    .set(f_comic_next_index.eq(f_comic_next_index + 1))
    .returning(f_comic_next_index - 1)
    .get_result::<i32>(conn)
    .await
    .optional()
    .map_err(diesel)?;

    let Some(index) = index else {
        return Err(missing_workset(id, "allocate workset comic index"));
    };

    accept!(usize_from_i32(index, "t_workset.f_comic_next_index")?)
}

// Update comic count by a delta value.
#[instrument(level = "info", skip_all)]
async fn update_comic_count(
    conn: &mut RdbConn,
    id: &str,
    delta: i32,
) -> BaseRest<()> {
    //
    // Keep a monotonic counter aligned with comic membership updates.
    let updated_count = diesel::update(
        t_workset.filter(f_id.eq(id)).filter(f_deleted_at.is_null()),
    )
    .set(f_comic_count.eq(f_comic_count + delta))
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_count == 0 {
        return Err(missing_workset(id, "update workset comic count"));
    }

    accept!(())
}

impl Run<GetWorksetInfo<'_>> for RdbRepo {
    // Use BaseError to keep run-level errors consistent.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Map `GetWorksetInfo` lookup to one repository query helper.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &GetWorksetInfo<'_>) -> BaseRest<WorksetInfo> {
        submit_query!(self.rdb_core, get_info, oper.id)
    }
}

impl Run<ListWorksetInfos<'_>> for RdbRepo {
    // Use BaseError to keep run-level errors consistent.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Map list request into paged, team-scoped query helper.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ListWorksetInfos<'_>,
    ) -> BaseRest<Vec<WorksetInfo>> {
        submit_query!(self.rdb_core, list_infos, oper)
    }
}

impl Run<UpdateWorkset<'_>> for RdbRepo {
    // Use BaseError to keep run-level errors consistent.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Route update DTO directly into update helper.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &UpdateWorkset<'_>) -> BaseRest<()> {
        submit_query!(self.rdb_core, update_info, oper.update)
    }
}

impl<L> Step<GetWorksetInfo<'_>, RdbContext<L>> for RdbRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use BaseError for transactional context operations.
    type Level = ReptRead;

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

impl<L> Step<ListWorksetInfos<'_>, RdbContext<L>> for RdbRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use BaseError for transactional context operations.
    type Level = ReptRead;

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

impl<L> Step<CreateWorkset<'_>, RdbContext<L>> for RdbRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use BaseError for transactional context operations.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Create workset record and return inserted info representation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateWorkset<'_>,
    ) -> BaseRest<WorksetInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<L> Step<AllocWorksetComicIndex<'_>, RdbContext<L>> for RdbRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use BaseError for transactional context operations.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Allocate and return the next comic index for the workset.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &AllocWorksetComicIndex<'_>,
    ) -> BaseRest<usize> {
        alloc_comic_index(context.conn(), oper.id).await
    }
}

impl<L> Step<UpdateWorksetComicCount<'_>, RdbContext<L>> for RdbRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use BaseError for transactional context operations.
    type Level = ReptRead;

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
