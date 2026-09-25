//! Complete Chapter creation inside a caller-owned transaction.

use poprako_orchestra::{AtLeast, Context, OperStep as _};

use crate::complex::{
    assignment as assignment_complex, chapter as chapter_complex,
};
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::shared::user::UserToken;
use crate::model::write::assignment::AssignmentEntry;
use crate::model::write::chapter::ChapterEntry;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::CreateAssignment;
use crate::part::repo::oper::chapter::{CreateChapter, UnpinOtherChapters};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::{
    AllocComicChapterIndex, TouchComicLastActive, UpdateComicChapterCount,
};
use crate::result::{BaseRest, accept};
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;
use crate::value::role::RoleMask;

/// Inputs for creating a pinned Chapter within an existing transaction.
pub struct ChapterCreation<'a> {
    /// Comic that owns the new Chapter.
    comic_info: &'a ComicInfo,
    /// Previously pinned Chapter, when one exists.
    prev_pinned_chapter: Option<ChapterInfo>,

    /// User creating the Chapter.
    token: &'a UserToken,

    /// Requested subtitle for the new Chapter.
    subtitle: Option<String>,
    /// Roles to assign to the creator, when requested.
    preset_assignment_roles: Option<RoleMask>,
}

impl<'a> ChapterCreation<'a> {
    /// Collects authorized creation inputs for a Chapter.
    pub const fn new(
        comic_info: &'a ComicInfo,
        prev_pinned_chapter: Option<ChapterInfo>,
        token: &'a UserToken,
        subtitle: Option<String>,
        preset_assignment_roles: Option<RoleMask>,
    ) -> Self {
        //
        Self {
            comic_info,
            prev_pinned_chapter,
            token,
            subtitle,
            preset_assignment_roles,
        }
    }
}

/// Creates a pinned Chapter and its counters, activity, assignment, and history.
///
/// The caller authorizes creation, validates preset roles, and owns the
/// transaction. For an existing Comic it also checks writability and locks the
/// Comic and its Chapters before observing the previous pin. A newly inserted
/// Comic has no previous pinned Chapter.
pub async fn create<C, R>(
    repo: &R,
    context: &mut C,
    creation: ChapterCreation<'_>,
) -> BaseRest<ChapterInfo>
where
    C: Context,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + Sync,
{
    let ChapterCreation {
        comic_info,
        prev_pinned_chapter,
        token,
        subtitle,
        preset_assignment_roles,
    } = creation;

    let index = AllocComicChapterIndex { id: &comic_info.id }
        .step_on(repo, context)
        .await?;

    let subtitle = chapter_complex::subtitle_or_default(subtitle, index);

    let chapter_id = chapter_complex::gen_id();

    UnpinOtherChapters {
        comic_id: &comic_info.id,
        excluded_id: &chapter_id,
    }
    .step_on(repo, context)
    .await?;

    let chapter_entry = ChapterEntry {
        id: chapter_id,
        comic_id: comic_info.id.as_str().into(),
        is_pinned: true,
        index,
        subtitle,
        creator_id: token.user_id.as_str().into(),
    };

    let chapter_info = CreateChapter {
        entry: &chapter_entry,
    }
    .step_on(repo, context)
    .await?;

    UpdateComicChapterCount {
        id: &chapter_info.comic_id,
        delta: 1,
    }
    .step_on(repo, context)
    .await?;

    TouchComicLastActive {
        id: &chapter_info.comic_id,
    }
    .step_on(repo, context)
    .await?;

    if let Some(roles) = preset_assignment_roles {
        //
        let assignment_entry = AssignmentEntry {
            id: assignment_complex::gen_id(),
            chapter_id: chapter_info.id.as_str().into(),
            user_id: token.user_id.as_str().into(),
            roles,
        };

        CreateAssignment {
            entry: &assignment_entry,
        }
        .step_on(repo, context)
        .await?;
    }

    let prev_pinned_chapter_id = prev_pinned_chapter
        .as_ref()
        .map(|chapter_info| chapter_info.id.as_str());

    record_created_chapter(
        repo,
        context,
        &token.user_id,
        prev_pinned_chapter_id,
        &chapter_info.id,
    )
    .await?;

    accept(chapter_info)
}

// Records creation and any displaced pinned chapter in the workflow history.
async fn record_created_chapter<C, R>(
    repo: &R,
    context: &mut C,
    user_id: &str,
    prev_pinned_chapter_id: Option<&str>,
    chapter_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: ChapterWorkflowRecordRepo<C> + Sync,
{
    let mut workflow_record_entries = Vec::with_capacity(2);

    if let Some(prev_pinned_chapter_id) = prev_pinned_chapter_id {
        //
        workflow_record_entries.push(ChapterWorkflowRecordEntry::new(
            prev_pinned_chapter_id,
            Some(user_id.into()),
            ChapterWorkflowRecordPayload::ChapterUnpinned,
        ));
    }

    workflow_record_entries.push(ChapterWorkflowRecordEntry::new(
        chapter_id,
        Some(user_id.into()),
        ChapterWorkflowRecordPayload::ChapterCreated,
    ));

    CreateChapterWorkflowRecords {
        entries: &workflow_record_entries,
    }
    .step_on(repo, context)
    .await?;

    accept(())
}
