//! Coordinated composite page write access.

use poprako_orchestra::{Context, OperStep as _};

use crate::complex::page_artwork as page_artwork_complex;
use crate::complex::page_artwork::perm as page_artwork_perm_complex;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;
use crate::value::role::RoleField;

/// Loads administration or current Chapter worker evidence.
pub async fn ensure_user_can_write<C, R>(
    repo: &R,
    context: &mut C,
    chapter_id: &str,
    user_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: TeamRepo<C> + MemberRepo<C> + AssignmentRepo<C> + Sync,
{
    let member_info = MemberLoader::find_info_from_chapter(
        repo,
        LoadMode::Step { context },
        user_id,
        chapter_id,
    )
    .await?;

    if member_info
        .is_some_and(|info| info.roles.has_any_role(&[RoleField::ADMIN]))
    {
        return accept(());
    }

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id,
    }
    .step_on(repo, context)
    .await?
    .ok_or_else(|| {
        //
        page_artwork_complex::error(
            ExpectedVariant::Perm,
            "error-page-artwork-write-role-required",
        )
    })?;

    page_artwork_perm_complex::ensure_user_can_write(&assignment_info)
}
