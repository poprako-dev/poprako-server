//! Shared helpers for the complex layer.

use poprako_util::i18n::trl;

use crate::part::repo::step::assignment::{AssignmentStep, GetInfoByChapterIdAndUserId};
use crate::part::repo::step::chapter::{ChapterStep, GetInfoById as ChapterGetInfoById};
use crate::part::repo::step::comic::{ComicStep, GetInfoById as ComicGetInfoById};
use crate::part::repo::step::member::{FindInfoByUserIdAndTeamId, MemberStep};
use crate::part::repo::step::workset::{GetInfoById as WorksetGetInfoById, WorksetStep};
use crate::part::shared::proxy::ProxyExecute;
use crate::result::{ExpectedVariant, RegularError, RegularResult, accept};
use crate::value::role::RoleField;

/// Verify the user is a member of the given team; returns `Perm` error if not.
pub async fn check_user_is_team_member<P>(
    proxy: &mut P,
    user_id: &str,
    team_id: &str,
) -> RegularResult<()>
where
    P: for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RegularError>,
{
    let member_info = proxy
        .execute(&MemberStep::find_info_by_user_id_and_team_id(
            user_id, team_id,
        ))
        .await?;

    if member_info.is_none() {
        return Err(RegularError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-member-required"),
        });
    }

    accept(())
}

/// Verify the user is a team admin; returns `Perm` error if not.
pub async fn check_user_is_team_admin<P>(
    proxy: &mut P,
    user_id: &str,
    team_id: &str,
) -> RegularResult<()>
where
    P: for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RegularError>,
{
    let member_info = proxy
        .execute(&MemberStep::find_info_by_user_id_and_team_id(
            user_id, team_id,
        ))
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

    accept(())
}

/// Verify the user is a team member for the chapter's owning team.
pub async fn check_user_is_team_member_by_chapter<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> RegularResult<()>
where
    P: for<'a> ProxyExecute<ChapterGetInfoById<'a>, Error = RegularError>
        + for<'a> ProxyExecute<ComicGetInfoById<'a>, Error = RegularError>
        + for<'a> ProxyExecute<WorksetGetInfoById<'a>, Error = RegularError>
        + for<'a> ProxyExecute<FindInfoByUserIdAndTeamId<'a>, Error = RegularError>,
{
    let chapter_info = proxy
        .execute(&ChapterStep::get_info_by_id(chapter_id, &[]))
        .await?;

    let comic_info = proxy
        .execute(&ComicStep::get_info_by_id(&chapter_info.comic_id, &[]))
        .await?;

    let workset_info = proxy
        .execute(&WorksetStep::get_info_by_id(&comic_info.workset_id))
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
    P: for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RegularError>,
{
    let assignment_info = proxy
        .execute(&AssignmentStep::get_info_by_chapter_id_and_user_id(
            chapter_id, user_id,
        ))
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
) -> RegularResult<()>
where
    P: for<'a> ProxyExecute<GetInfoByChapterIdAndUserId<'a>, Error = RegularError>,
{
    let assignment_info = proxy
        .execute(&AssignmentStep::get_info_by_chapter_id_and_user_id(
            chapter_id, user_id,
        ))
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

fn chapter_assignee_required_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-assignee-required"),
    }
}

fn chapter_translator_or_proofreader_required_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-translator-or-proofreader-required"),
    }
}
