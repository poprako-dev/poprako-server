//! Shared authorization under the caller's chapter lock.

use poprako_orchestra::{Context, OperStep as _};

use crate::complex::issue::perm as issue_perm_complex;
use crate::complex::{chapter as chapter_complex, issue as issue_complex};
use crate::model::read::proj::chapter::ChapterInfo;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::result::{BaseRest, ExpectedVariant};

/// Authorizes a chapter write.
pub async fn write<C, R>(
    repo: &R,
    context: &mut C,
    chapter_id: &str,
    user_id: &str,
) -> BaseRest<ChapterInfo>
where
    C: Context + Send,
    R: ChapterRepo<C> + AssignmentRepo<C> + Sync,
{
    let chapter_info = GetChapterInfoExcluded {
        id: chapter_id,
        incls: &[],
    }
    .step_on(repo, context)
    .await?;

    chapter_complex::ensure_chapter_writable(&chapter_info)?;

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id,
    }
    .step_on(repo, context)
    .await?
    .ok_or_else(|| {
        //
        issue_complex::error(
            ExpectedVariant::Perm,
            "error-issue-reviewer-required",
        )
    })?;

    issue_perm_complex::ensure_user_can_import_issue(&assignment_info)?;

    accept!(chapter_info)
}
