//! Mock immutable chapter workflow record repository operations.

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::chapter_workflow_record::ChapterWorkflowRecordInfo;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::chapter_workflow_record::{
    CreateChapterWorkflowRecords, DeleteChapterWorkflowRecords,
    ListChapterWorkflowRecordInfos, ListChapterWorkflowRecordInfosExcluded,
};
use crate::part_impl::repo::mock_impl::{Mock, MockContext, MockState};
use crate::result::{BaseError, BaseRest, accept};

// List immutable records using the API's deterministic reverse chronological order.
fn list_infos(
    state: &MockState,
    chapter_id: &str,
    offset: u32,
    limit: u32,
) -> Vec<ChapterWorkflowRecordInfo> {
    //
    let mut record_infos = state
        .chapter_workflow_records
        .iter()
        .filter(|record_info| record_info.chapter_id == chapter_id)
        .cloned()
        .collect::<Vec<_>>();

    record_infos.sort_by(|left, right| {
        //
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.id.cmp(&left.id))
    });

    let offset = offset as usize;

    match offset >= record_infos.len() {
        //
        true => Vec::new(),

        false => {
            //
            let end =
                std::cmp::min(offset + limit as usize, record_infos.len());

            record_infos[offset..end].to_vec()
        }
    }
}

impl<'a> Run<ListChapterWorkflowRecordInfos<'a>> for Mock {
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Lists a deterministic page from the mock state snapshot.
    async fn run(
        &self,
        oper: &ListChapterWorkflowRecordInfos<'a>,
    ) -> BaseRest<Vec<ChapterWorkflowRecordInfo>> {
        //
        let state = self.state.lock().unwrap();

        accept(list_infos(
            &state,
            &oper.spec.chapter_id,
            oper.spec.offset,
            oper.spec.limit,
        ))
    }
}

impl<'a> Step<CreateChapterWorkflowRecords<'a>, MockContext> for Mock {
    // Declares the transaction isolation level required for inserts.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Appends immutable records to the transaction-local mock state.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateChapterWorkflowRecords<'a>,
    ) -> BaseRest<()> {
        //
        context
            .state
            .chapter_workflow_records
            .extend(oper.entries.iter().map(|entry| {
                //
                ChapterWorkflowRecordInfo {
                    id: entry.id.clone(),
                    chapter_id: entry.chapter_id.clone(),
                    actor_user_id: entry.actor_user_id.clone(),
                    kind: entry.payload.kind(),
                    payload: entry.payload.clone(),
                    created_at: entry.created_at,
                }
            }));

        accept(())
    }
}

impl<'a> Step<ListChapterWorkflowRecordInfosExcluded<'a>, MockContext>
    for Mock
{
    // Declares the transaction isolation level required for locked reads.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Returns archive-ordered records from the transaction-local state.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListChapterWorkflowRecordInfosExcluded<'a>,
    ) -> BaseRest<Vec<ChapterWorkflowRecordInfo>> {
        //
        let mut record_infos = context
            .state
            .chapter_workflow_records
            .iter()
            .filter(|record_info| record_info.chapter_id == oper.chapter_id)
            .cloned()
            .collect::<Vec<_>>();

        record_infos.sort_by(|left, right| {
            //
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });

        accept(record_infos)
    }
}

impl<'a> Step<DeleteChapterWorkflowRecords<'a>, MockContext> for Mock {
    // Declares the transaction isolation level required for deletion.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Removes active records for the deleted chapter from the mock state.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteChapterWorkflowRecords<'a>,
    ) -> BaseRest<()> {
        //
        context
            .state
            .chapter_workflow_records
            .retain(|record_info| record_info.chapter_id != oper.chapter_id);

        accept(())
    }
}
