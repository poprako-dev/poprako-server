//! Repository capability for immutable chapter workflow records.

use poprako_orchestra::drive;

use crate::part::repo::oper::chapter_workflow_record::{
    CreateChapterWorkflowRecords, DeleteChapterWorkflowRecords,
    ListChapterWorkflowRecordInfos, ListChapterWorkflowRecordInfosExcluded,
};
use crate::result::BaseError;

/// Immutable chapter workflow record repository operations.
#[drive(
    context = C,
    error = BaseError,
    run(for<'a> ListChapterWorkflowRecordInfos<'a>),
    step(
        for<'a> CreateChapterWorkflowRecords<'a>,
        for<'a> ListChapterWorkflowRecordInfosExcluded<'a>,
        for<'a> DeleteChapterWorkflowRecords<'a>,
    ),
)]
pub trait ChapterWorkflowRecordRepo<C> {}
