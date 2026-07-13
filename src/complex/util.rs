//! Shared helpers for the complex layer.

use poprako_orchestra::Proxy;

use poprako_util::i18n::trl;

use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::result::{ExpectedVariant, RegularError, RegularResult};
use crate::value::role::RoleField;

/// Verify the user is a member of the given team; returns `Perm` error if not.
pub async fn check_user_is_team_member<P>(
    proxy: &mut P,
    user_id: &str,
    team_id: &str,
) -> RegularResult<()>
where
    P: for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
{
    let member_info = proxy
        .exec(&FindMemberInfo::UserTeam { user_id, team_id })
        .await?;

    if member_info.is_none() {
        return Err(RegularError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-member-required"),
        });
    }

    Ok(())
}

/// Verify the user is a team admin; returns `Perm` error if not.
pub async fn check_user_is_team_admin<P>(
    proxy: &mut P,
    user_id: &str,
    team_id: &str,
) -> RegularResult<()>
where
    P: for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
{
    let member_info = proxy
        .exec(&FindMemberInfo::UserTeam { user_id, team_id })
        .await?;

    let Some(member_info) = member_info else {
        return Err(RegularError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    };

    if !member_info.roles.has_any_role(&[RoleField::ADMIN]) {
        return Err(RegularError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    }

    Ok(())
}

/// Verify the user is a team member for the chapter's owning team.
pub async fn check_user_is_team_member_by_chapter<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> RegularResult<()>
where
    P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = RegularError>
        + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = RegularError>
        + for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
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
) -> RegularResult<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = RegularError>,
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

    Ok(())
}

/// Verify the user is assigned as translator or proofreader on the chapter.
pub async fn check_user_is_chapter_translator_or_proofreader<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> RegularResult<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = RegularError>,
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

    Ok(())
}

/// Construct a "chapter assignee required" permission error.
fn chapter_assignee_required_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-assignee-required"),
    }
}

/// Construct a "translator or proofreader required" permission error.
fn chapter_translator_or_proofreader_required_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-translator-or-proofreader-required"),
    }
}
