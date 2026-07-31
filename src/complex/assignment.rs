//! Complex-domain opers for chapter assignments.

use poprako_orchestra::{OperProxy as _, Proxy};

use poprako_util::i18n::trl;

use crate::complex::util::check_user_is_team_member;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::spec::assignment::AssignmentListSpec;
use crate::model::write::assignment::AssignmentRoleRepl;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::user::GetUserInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::role::{RoleField, RoleMask};

/// Domain opers for chapter assignments: ID generation and role-merge logic.
pub struct AssignmentComplex;

impl AssignmentComplex {
    /// Generate a unique assignment identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Build the creator assignment roles, always preserving chapter admin.
    pub fn creator_roles(preset_roles: Option<RoleMask>) -> RoleMask {
        //
        let admin_roles = RoleMask::from(RoleField::ADMIN);

        preset_roles
            .map(|roles| roles.union(admin_roles))
            .unwrap_or(admin_roles)
    }

    /// Merge new roles into an existing assignment, preserving existing roles
    /// and writing new ones.
    pub fn merge_roles(
        assignment_info: &AssignmentInfo,
        roles: RoleMask,
    ) -> AssignmentRoleRepl {
        AssignmentRoleRepl {
            id: assignment_info.id.clone(),
            roles: assignment_info.roles.union(roles),
        }
    }

    /// Checks whether a role update would remove the caller's own admin role.
    pub fn is_self_admin_role_removal(
        current_user_id: &str,
        assignment_info: &AssignmentInfo,
        roles: RoleMask,
    ) -> bool {
        current_user_id == assignment_info.user_id
            && assignment_info.roles.has_any_role(&[RoleField::ADMIN])
            && !roles.has_any_role(&[RoleField::ADMIN])
    }

    /// Checks whether a chapter still has at least one admin after a role update.
    pub fn chapter_has_admin_after_role_update(
        assignment_infos: &[AssignmentInfo],
        user_id: &str,
        roles: RoleMask,
    ) -> bool {
        assignment_infos.iter().any(|assignment_info| {
            match assignment_info.user_id == user_id {
                //
                true => roles.has_any_role(&[RoleField::ADMIN]),

                false => {
                    assignment_info.roles.has_any_role(&[RoleField::ADMIN])
                }
            }
        })
    }
}

/// perm-gate opers for chapter assignments.
pub struct AssignmentPermComplex;

impl AssignmentPermComplex {
    /// Verify the caller may list assignments selected by the list spec.
    pub async fn ensure_user_can_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        assignment_list_spec: &AssignmentListSpec,
    ) -> BaseRest<()>
    where
        P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
            + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>
            + for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetUserInfo<'a>, Error = BaseError>,
    {
        match assignment_list_spec {
            //
            AssignmentListSpec::Chapter { chapter_id, .. } => {
                check_list_by_chapter(proxy, user_id, chapter_id).await
            }

            AssignmentListSpec::User { owner_id, .. } => {
                check_list_by_user(proxy, user_id, owner_id).await
            }
        }
    }

    /// Verify the caller may mutate assignment roles with the supplied data.
    pub async fn ensure_user_can_update_roles<P>(
        proxy: &mut P,
        current_user_id: &str,
        subject_user_id: &str,
        chapter_id: &str,
        roles: RoleMask,
    ) -> BaseRest<()>
    where
        P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
            + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>
            + for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
    {
        let admin_check = check_admin(proxy, current_user_id, chapter_id).await;

        if admin_check.is_err() {
            check_self_reduce(
                proxy,
                current_user_id,
                subject_user_id,
                chapter_id,
                roles,
            )
            .await?;
        }

        check_target_roles(proxy, subject_user_id, chapter_id, roles).await
    }

    /// Verify the caller may delete the target assignment.
    pub async fn ensure_user_can_delete<P>(
        proxy: &mut P,
        current_user_id: &str,
        assignment_info: &AssignmentInfo,
    ) -> BaseRest<()>
    where
        P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
    {
        if current_user_id == assignment_info.user_id {
            return accept(());
        }

        check_admin(proxy, current_user_id, &assignment_info.chapter_id).await
    }

    /// Verify the target user may take the requested chapter assignment roles.
    pub async fn ensure_user_can_take_roles<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
        roles: RoleMask,
    ) -> BaseRest<()>
    where
        P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
            + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
    {
        check_target_roles(proxy, user_id, chapter_id, roles).await
    }
}

// Resolve the owning team ID from a chapter ID via its comic and workset.
async fn resolve_team_id<P>(proxy: &mut P, chapter_id: &str) -> BaseRest<String>
where
    P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
        + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
        + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>,
{
    let chapter_info = GetChapterInfo {
        id: chapter_id,
        incls: &[],
    }
    .proxy_on(proxy)
    .await?;

    let comic_info = GetComicInfo {
        id: &chapter_info.comic_id,
        incls: &[],
    }
    .proxy_on(proxy)
    .await?;

    let workset_info = GetWorksetInfo {
        id: &comic_info.workset_id,
    }
    .proxy_on(proxy)
    .await?;

    accept(workset_info.team_id)
}

// Verify the caller may list assignments for a chapter as a team
// member of the owning team, or as a chapter assignee.
async fn check_list_by_chapter<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> BaseRest<()>
where
    P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
        + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
        + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>
        + for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
{
    let team_id = resolve_team_id(proxy, chapter_id).await?;

    let member_check =
        check_user_is_team_member(proxy, user_id, &team_id).await;

    if member_check.is_ok() {
        return accept(());
    }

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id,
    }
    .proxy_on(proxy)
    .await?;

    if assignment_info.is_none() {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            team_id = %team_id,
            "expected error: assignment list perm denied",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}

// Verify the caller may list assignments for a user as the owner
// or as a super-admin.
async fn check_list_by_user<P>(
    proxy: &mut P,
    current_user_id: &str,
    owner_id: &str,
) -> BaseRest<()>
where
    P: for<'a> Proxy<GetUserInfo<'a>, Error = BaseError>,
{
    if current_user_id == owner_id {
        return accept(());
    }

    let user_info = GetUserInfo::Id {
        id: current_user_id,
    }
    .proxy_on(proxy)
    .await?;

    if !user_info.is_sadmin {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            current_user_id = %current_user_id,
            owner_id = %owner_id,
            "expected error: assignment list perm denied",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}

// Verify the caller is assigned as a chapter admin on this chapter.
async fn check_admin<P>(
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
        let err_message = trl("error-chapter-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            "expected error: chapter admin assignment missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    if !assignment_info.roles.has_any_role(&[RoleField::ADMIN]) {
        //
        let err_message = trl("error-chapter-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            assignment_roles = ?assignment_info.roles,
            "expected error: chapter admin role missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}

// Verify the caller is reducing their own admin role assignment.
// The caller must be the target user and currently hold the roles they
// are removing.
async fn check_self_reduce<P>(
    proxy: &mut P,
    current_user_id: &str,
    subject_user_id: &str,
    chapter_id: &str,
    roles: RoleMask,
) -> BaseRest<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>,
{
    if current_user_id != subject_user_id {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            current_user_id = %current_user_id,
            subject_user_id = %subject_user_id,
            chapter_id = %chapter_id,
            roles = ?roles,
            "expected error: assignment self-reduce denied",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id: subject_user_id,
    }
    .proxy_on(proxy)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            current_user_id = %current_user_id,
            subject_user_id = %subject_user_id,
            chapter_id = %chapter_id,
            roles = ?roles,
            "expected error: assignment self-reduce target missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    if !assignment_info.roles.contains_mask(roles) {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            current_user_id = %current_user_id,
            subject_user_id = %subject_user_id,
            chapter_id = %chapter_id,
            roles = ?roles,
            assignment_roles = ?assignment_info.roles,
            "expected error: assignment self-reduce roles not held",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}

// Verify the target user's team membership permits the requested roles.
// Also reject `ADMIN` roles because they are not assignable through this flow.
async fn check_target_roles<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
    roles: RoleMask,
) -> BaseRest<()>
where
    P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = BaseError>
        + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = BaseError>
        + for<'a> Proxy<GetWorksetInfo<'a>, Error = BaseError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    if roles.has_any_role(&[RoleField::ADMIN]) {
        //
        let err_message = trl("error-chapter-role-not-assignable");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            roles = ?roles,
            "expected error: admin role is not assignable",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    let team_id = resolve_team_id(proxy, chapter_id).await?;

    let member_info = FindMemberInfo::UserTeam {
        user_id,
        team_id: &team_id,
    }
    .proxy_on(proxy)
    .await?;

    let Some(member_info) = member_info else {
        //
        let err_message = trl("error-chapter-role-not-assignable");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            team_id = %team_id,
            roles = ?roles,
            "expected error: target member is missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    if !member_info.roles.contains_mask(roles) {
        //
        let err_message = trl("error-chapter-role-not-assignable");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            team_id = %team_id,
            roles = ?roles,
            member_roles = ?member_info.roles,
            "expected error: target member lacks requested roles",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}
