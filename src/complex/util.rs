//! Shared helpers for the complex layer.

use poprako_orchestra::Proxy;

use poprako_util::i18n::trl;

use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::value::role::{RoleField, RoleMask};

/// Verify the user is a member of the given team; returns `Perm` error if not.
pub async fn check_user_is_team_member<P>(
    proxy: &mut P,
    user_id: &str,
    team_id: &str,
) -> BaseResult<()>
where
    P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    let member_info = proxy
        .exec(&FindMemberInfo::UserTeam { user_id, team_id })
        .await?;

    if member_info.is_none() {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-member-required"),
        });
    }

    accept(())
}

/// Verify the user is a team admin and owns every optionally required role.
pub async fn check_user_is_team_admin_with_roles<P>(
    proxy: &mut P,
    user_id: &str,
    team_id: &str,
    required_roles: Option<RoleMask>,
) -> BaseResult<()>
where
    P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    let member_info = proxy
        .exec(&FindMemberInfo::UserTeam { user_id, team_id })
        .await?;

    let Some(member_info) = member_info else {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    };

    if !member_info.roles.has_any_role(&[RoleField::ADMIN]) {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    }

    if required_roles
        .is_some_and(|roles| !member_info.roles.contains_mask(roles))
    {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-chapter-role-not-assignable"),
        });
    }

    accept(())
}

/// Verify the user is a team admin; returns `Perm` error if not.
pub async fn check_user_is_team_admin<P>(
    proxy: &mut P,
    user_id: &str,
    team_id: &str,
) -> BaseResult<()>
where
    P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    check_user_is_team_admin_with_roles(proxy, user_id, team_id, None).await
}

/// Verify the user is a team member for the chapter's owning team.
pub async fn check_user_is_team_member_by_chapter<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseResult<()>
where
    P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
        + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
        + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    let chapter_info = proxy
        .exec(&GetChapterInfo {
            id: chapter_id,
            incls: &[],
        })
        .await?;

    let comic_info = proxy
        .exec(&GetComicInfo {
            id: &chapter_info.comic_id,
            incls: &[],
        })
        .await?;

    let workset_info = proxy
        .exec(&GetWorksetInfo {
            id: &comic_info.workset_id,
        })
        .await?;

    check_user_is_team_member(proxy, user_id, &workset_info.team_id).await
}

/// Verify the user has an assignment on the chapter.
pub async fn check_user_is_chapter_assignee<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseResult<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
{
    let assignment_info = proxy
        .exec(&FindAssignmentInfo::ChapterUser {
            chapter_id,
            user_id,
        })
        .await?;

    if assignment_info.is_none() {
        return Err(chapter_assignee_required_error());
    }

    accept(())
}

/// Verify the user is assigned as translator or proofreader on the chapter.
pub async fn check_user_is_chapter_translator_or_proofreader<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseResult<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
{
    let assignment_info = proxy
        .exec(&FindAssignmentInfo::ChapterUser {
            chapter_id,
            user_id,
        })
        .await?;

    let Some(assignment_info) = assignment_info else {
        return Err(chapter_translator_or_proofreader_required_error());
    };

    if !assignment_info
        .roles
        .has_any_role(&[RoleField::TRANSLATOR, RoleField::PROOFREADER])
    {
        return Err(chapter_translator_or_proofreader_required_error());
    }

    accept(())
}

/// Construct a "chapter assignee required" permission error.
fn chapter_assignee_required_error() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-assignee-required"),
    }
}

/// Construct a "translator or proofreader required" permission error.
fn chapter_translator_or_proofreader_required_error() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-translator-or-proofreader-required"),
    }
}
