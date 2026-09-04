//! Mock immutable chapter workflow record repository operations.

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::chapter_workflow_record::ChapterWorkflowRecordInfo;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::chapter_workflow_record::{
    CreateChapterWorkflowRecords, ListChapterWorkflowRecordInfos,
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

    if offset >= record_infos.len() {
        Vec::new()
    } else {
        //
        let end = std::cmp::min(offset + limit as usize, record_infos.len());

        record_infos[offset..end].to_vec()
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
            oper.spec.limit.get(),
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
