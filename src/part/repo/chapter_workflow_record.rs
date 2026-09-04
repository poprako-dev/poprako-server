//! Repository capability for immutable chapter workflow records.

use poprako_orchestra::drive;

use crate::part::repo::oper::chapter_workflow_record::{
    CreateChapterWorkflowRecords, ListChapterWorkflowRecordInfos,
};
use crate::result::BaseError;

/// Immutable chapter workflow record repository operations.
#[drive(
    context = C,
    error = BaseError,
    run(for<'a> ListChapterWorkflowRecordInfos<'a>),
    step(for<'a> CreateChapterWorkflowRecords<'a>),
)]
pub trait ChapterWorkflowRecordRepo<C> {}
