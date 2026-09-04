//! RDB operations for immutable chapter workflow records.

/// RDB integration tests for immutable chapter workflow records.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Run, Step};
use tracing::instrument;

use poprako_rdb_core::RdbConn;

use crate::part::nucl::ReptRead;
use crate::model::read::proj::chapter_workflow_record::ChapterWorkflowRecordInfo;
use crate::model::read::spec::chapter_workflow_record::ChapterWorkflowRecordListSpec;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::repo::oper::chapter_workflow_record::{CreateChapterWorkflowRecords, ListChapterWorkflowRecordInfos};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::chapter_workflow_record::{ChapterWorkflowRecordEntryRow, ChapterWorkflowRecordInfoRow};
use crate::part_impl::repo::rdb_impl::schema::t_chapter_workflow_record::dsl::{f_chapter_id, f_created_at, f_id, t_chapter_workflow_record};
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::result::diesel;
use crate::shared::RdbContext;

// Lists one reverse-chronological page of immutable workflow records.
#[instrument(level = "info", skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &ChapterWorkflowRecordListSpec,
) -> BaseRest<Vec<ChapterWorkflowRecordInfo>> {
    //
    let rows = t_chapter_workflow_record
        .filter(f_chapter_id.eq(&spec.chapter_id))
        .order_by((f_created_at.desc(), f_id.desc()))
        .offset(i64::from(spec.offset))
        .limit(i64::from(spec.limit.get()))
        .select(ChapterWorkflowRecordInfoRow::as_select())
        .load::<ChapterWorkflowRecordInfoRow>(conn)
        .await
        .map_err(diesel)?;

    rows.into_iter().map(TryInto::try_into).collect()
}

// Inserts a batch of immutable workflow records in the active transaction.
#[instrument(level = "info", skip_all)]
async fn create(
    conn: &mut RdbConn,
    entries: &[ChapterWorkflowRecordEntry],
) -> BaseRest<()> {
    //
    if entries.is_empty() {
        return accept(());
    }

    let rows = entries
        .iter()
        .map(ChapterWorkflowRecordEntryRow::from)
        .collect::<Vec<_>>();

    diesel::insert_into(t_chapter_workflow_record)
        .values(&rows)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

impl Run<ListChapterWorkflowRecordInfos<'_>> for HybRepo {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Lists a page using an independent query connection.
    async fn run(
        &self,
        oper: &ListChapterWorkflowRecordInfos<'_>,
    ) -> BaseRest<Vec<ChapterWorkflowRecordInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl<L> Step<CreateChapterWorkflowRecords<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Declares the transaction isolation level required for inserts.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Inserts all records inside the caller-owned transaction.
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateChapterWorkflowRecords<'_>,
    ) -> BaseRest<()> {
        create(context.conn(), oper.entries).await
    }
}
