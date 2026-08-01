//! Shared helpers for the complex layer.

use poprako_orchestra::{OperProxy as _, Proxy};

use poprako_util::i18n::trl;

use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::team::ResolveTeamId;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::role::{RoleField, RoleMask};

/// Verify the user is a member of the given team; returns `Perm` error if not.
pub async fn check_user_is_team_member<P>(
    proxy: &mut P,
    user_id: &str,
    team_id: &str,
) -> BaseRest<()>
where
    P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    let member_info = FindMemberInfo::UserTeam { user_id, team_id }
        .proxy_on(proxy)
        .await?;

    if member_info.is_none() {
        //
        let err_message = trl("error-team-member-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            team_id = %team_id,
            "expected error: team membership required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}

/// Verify the user is a proofreader in the given team.
pub async fn check_user_is_team_proofreader<P>(
    proxy: &mut P,
    user_id: &str,
    team_id: &str,
) -> BaseRest<()>
where
    P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    let member_info = FindMemberInfo::UserTeam { user_id, team_id }
        .proxy_on(proxy)
        .await?;

    let Some(member_info) = member_info else {
        //
        let err_message = trl("error-team-proofreader-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            team_id = %team_id,
            "expected error: team proofreader membership missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    if !member_info.roles.has_any_role(&[RoleField::PROOFREADER]) {
        //
        let err_message = trl("error-team-proofreader-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            team_id = %team_id,
            member_roles = ?member_info.roles,
            "expected error: team proofreader role missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
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
) -> BaseRest<()>
where
    P: for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    let member_info = FindMemberInfo::UserTeam { user_id, team_id }
        .proxy_on(proxy)
        .await?;

    let Some(member_info) = member_info else {
        //
        let err_message = trl("error-team-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            team_id = %team_id,
            required_roles = ?required_roles,
            "expected error: team admin membership missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    if !member_info.roles.has_any_role(&[RoleField::ADMIN]) {
        //
        let err_message = trl("error-team-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            team_id = %team_id,
            member_roles = ?member_info.roles,
            required_roles = ?required_roles,
            "expected error: team admin role missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    if required_roles
        .is_some_and(|roles| !member_info.roles.contains_mask(roles))
    {
        let err_message = trl("error-chapter-role-not-assignable");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            team_id = %team_id,
            required_roles = ?required_roles,
            member_roles = ?member_info.roles,
            "expected error: team member lacks required roles",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}

/// Verify the user is a team admin; returns `Perm` error if not.
pub async fn check_user_is_team_admin<P>(
    proxy: &mut P,
    user_id: &str,
    team_id: &str,
) -> BaseRest<()>
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
) -> BaseRest<()>
where
    P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    let team_id = ResolveTeamId::Chapter { id: chapter_id }
        .proxy_on(proxy)
        .await?;

    check_user_is_team_member(proxy, user_id, &team_id).await
}

/// Verify the user has an assignment on the chapter.
pub async fn check_user_is_chapter_assignee<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseRest<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id,
    }
    .proxy_on(proxy)
    .await?;

    if assignment_info.is_none() {
        //
        let err_message = trl("error-chapter-assignee-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            "expected error: chapter assignee required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}

/// Verify the user is assigned as translator or proofreader on the chapter.
pub async fn check_user_is_chapter_translator_or_proofreader<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseRest<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id,
    }
    .proxy_on(proxy)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        let err_message =
            trl("error-chapter-translator-or-proofreader-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            "expected error: chapter translator or proofreader assignment missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    if !assignment_info
        .roles
        .has_any_role(&[RoleField::TRANSLATOR, RoleField::PROOFREADER])
    {
        let err_message =
            trl("error-chapter-translator-or-proofreader-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            assignment_roles = ?assignment_info.roles,
            "expected error: chapter translator or proofreader role missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}
