//! Transactional permission enforcement shared by source allocation flows.

use poprako_orchestra::{Context, OperStep as _};

use poprako_util::i18n::trl;

use crate::complex::page_source::perm as page_source_perm_complex;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::result::{BaseRest, ExpectedVariant};

/// Enforces allocation permission inside the caller's transaction.
pub async fn ensure_alloc_perm<C, R>(
    repo: &R,
    context: &mut C,
    user_id: &str,
    chapter_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: AssignmentRepo<C> + Sync,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id,
    }
    .step_on(repo, context)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        return Err(expected_error! {
            ExpectedVariant::Perm,
            message: trl("error-page-alloc-role-required");
            "page allocation role required",
        });
    };

    page_source_perm_complex::ensure_user_can_alloc(&assignment_info)
}
