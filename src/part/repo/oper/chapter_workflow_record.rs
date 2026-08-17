//! Repository operations for immutable chapter workflow records.

use poprako_orchestra::Oper;

use crate::model::read::proj::chapter_workflow_record::ChapterWorkflowRecordInfo;
use crate::model::read::spec::chapter_workflow_record::ChapterWorkflowRecordListSpec;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;

/// Lists a page of workflow records for one chapter in reverse chronological order.
#[derive(Oper)]
#[oper(output = Vec<ChapterWorkflowRecordInfo>)]
pub struct ListChapterWorkflowRecordInfos<'a> {
    /// Chapter and pagination selection.
    pub spec: &'a ChapterWorkflowRecordListSpec,
}

/// Inserts one or more immutable workflow records within the caller's transaction.
#[derive(Oper)]
#[oper(output = ())]
pub struct CreateChapterWorkflowRecords<'a> {
    /// Entries to persist together.
    pub entries: &'a [ChapterWorkflowRecordEntry],
}

/// Locks and lists a chapter's workflow records in archive order.
#[derive(Oper)]
#[oper(output = Vec<ChapterWorkflowRecordInfo>)]
pub struct ListChapterWorkflowRecordInfosExcluded<'a> {
    /// Chapter whose records are locked and loaded.
    pub chapter_id: &'a str,
}

/// Deletes all active workflow records for a chapter.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteChapterWorkflowRecords<'a> {
    /// Chapter whose records are deleted.
    pub chapter_id: &'a str,
}
