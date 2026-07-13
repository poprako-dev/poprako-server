//! Complex-domain opers for chapter assignments.

use poprako_orchestra::Proxy;

use poprako_util::i18n::trl;

use crate::complex::util::check_user_is_team_member;
use crate::data::assignment::UpdateAssignmentRolesParams;
use crate::model::assignment::{
    AssignmentInfo, AssignmentInfoListSpec, AssignmentRoleUpdate,
};
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::user::GetUserInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::result::{ExpectedVariant, RegularError, RegularResult};
use crate::util::next_snowflake_id;
use crate::value::role::{RoleField, RoleMask};

/// Domain opers for chapter assignments: ID generation and role-merge logic.
pub struct AssignmentComplex;

impl AssignmentComplex {
    /// Generate a unique assignment identifier backed by a snowflake value.
    pub fn gen_id() -> String {
        next_snowflake_id()
    }

    /// Merge new roles into an existing assignment, preserving existing roles
    /// and writing new ones.
    pub fn merge_roles(
        assignment_info: &AssignmentInfo,
        roles: RoleMask,
    ) -> AssignmentRoleUpdate {
        AssignmentRoleUpdate {
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

/// Permission-gate opers for chapter assignments.
pub struct AssignmentPermComplex;

impl AssignmentPermComplex {
    /// Verify the caller may list assignments selected by the list spec.
    pub async fn ensure_user_can_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        assignment_list_spec: &AssignmentInfoListSpec,
    ) -> RegularResult<()>
    where
        P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = RegularError>
            + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = RegularError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>
            + for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = RegularError>
            + for<'a> Proxy<GetUserInfo<'a>, Error = RegularError>,
    {
        match assignment_list_spec {
            //
            AssignmentInfoListSpec::Chapter { chapter_id, .. } => {
                check_list_by_chapter(proxy, user_id, chapter_id).await
            }

            AssignmentInfoListSpec::User { owner_id, .. } => {
                check_list_by_user(proxy, user_id, owner_id).await
            }
        }
    }

    /// Verify the caller may mutate assignment roles with the supplied data.
    pub async fn ensure_user_can_update_roles<P>(
        proxy: &mut P,
        current_user_id: &str,
        data: &UpdateAssignmentRolesParams,
    ) -> RegularResult<()>
    where
        P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = RegularError>
            + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = RegularError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>
            + for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = RegularError>,
    {
        let admin_check =
            check_admin(proxy, current_user_id, &data.chapter_id).await;

        if admin_check.is_err() {
            check_self_reduce(proxy, current_user_id, data).await?;
        }

        check_target_roles(proxy, &data.user_id, &data.chapter_id, data.roles)
            .await
    }

    /// Verify the caller may delete the target assignment.
    pub async fn ensure_user_can_delete<P>(
        proxy: &mut P,
        current_user_id: &str,
        assignment_info: &AssignmentInfo,
    ) -> RegularResult<()>
    where
        P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = RegularError>,
    {
        if current_user_id == assignment_info.user_id {
            return Ok(());
        }

        check_admin(proxy, current_user_id, &assignment_info.chapter_id).await
    }

    /// Verify the caller is an admin for the target chapter.
    pub async fn ensure_user_can_admin<P>(
        proxy: &mut P,
        current_user_id: &str,
        chapter_id: &str,
    ) -> RegularResult<()>
    where
        P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = RegularError>,
    {
        check_admin(proxy, current_user_id, chapter_id).await
    }

    /// Verify the target user may take the requested chapter assignment roles.
    pub async fn ensure_user_can_take_roles<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
        roles: RoleMask,
    ) -> RegularResult<()>
    where
        P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = RegularError>
            + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = RegularError>
            + for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>
            + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
    {
        check_target_roles(proxy, user_id, chapter_id, roles).await
    }
}

/// Verify the caller may list assignments for a chapter — either as a team
/// member of the owning team, or as a chapter assignee.
async fn check_list_by_chapter<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
) -> RegularResult<()>
where
    P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = RegularError>
        + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = RegularError>
        + for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>
        + for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = RegularError>,
{
    let team_id = resolve_team_id(proxy, chapter_id).await?;

    let member_check =
        check_user_is_team_member(proxy, user_id, &team_id).await;

    if member_check.is_ok() {
        return Ok(());
    }

    let assignment_info = proxy
        .exec(&FindAssignmentInfo::ChapterUser {
            chapter_id,
            user_id,
        })
        .await?;

    if assignment_info.is_none() {
        return Err(assignment_list_permission_error());
    }

    Ok(())
}

/// Verify the caller may list assignments for a user — either as the owner
/// or as a super-admin.
async fn check_list_by_user<P>(
    proxy: &mut P,
    current_user_id: &str,
    owner_id: &str,
) -> RegularResult<()>
where
    P: for<'a> Proxy<GetUserInfo<'a>, Error = RegularError>,
{
    if current_user_id == owner_id {
        return Ok(());
    }

    let user_info = proxy
        .exec(&GetUserInfo::Id {
            id: current_user_id,
        })
        .await?;

    if !user_info.is_sadmin {
        return Err(assignment_list_permission_error());
    }

    Ok(())
}

/// Verify the caller is assigned as a chapter admin on this chapter.
async fn check_admin<P>(
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
        return Err(chapter_admin_error());
    };

    if !assignment_info.roles.has_any_role(&[RoleField::ADMIN]) {
        return Err(chapter_admin_error());
    }

    Ok(())
}

/// Verify the caller is reducing their own admin role assignment — the
/// caller must be the target user and must currently hold the roles they
/// are removing.
async fn check_self_reduce<P>(
    proxy: &mut P,
    current_user_id: &str,
    data: &UpdateAssignmentRolesParams,
) -> RegularResult<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = RegularError>,
{
    if current_user_id != data.user_id {
        return Err(assignment_self_reduce_error());
    }

    let assignment_info = proxy
        .exec(&FindAssignmentInfo::ChapterUser {
            chapter_id: &data.chapter_id,
            user_id: &data.user_id,
        })
        .await?;

    let Some(assignment_info) = assignment_info else {
        return Err(assignment_self_reduce_error());
    };

    if !assignment_info.roles.contains_mask(data.roles) {
        return Err(assignment_self_reduce_error());
    }

    Ok(())
}

/// Verify the target user's team membership permits the requested role bits.
/// Also rejects `ADMIN` roles (not assignable through the update flow).
async fn check_target_roles<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
    roles: RoleMask,
) -> RegularResult<()>
where
    P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = RegularError>
        + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = RegularError>
        + for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = RegularError>,
{
    if roles.has_any_role(&[RoleField::ADMIN]) {
        return Err(assignment_role_not_assignable_args_error());
    }

    let team_id = resolve_team_id(proxy, chapter_id).await?;

    let member_info = proxy
        .exec(&FindMemberInfo::UserTeam {
            user_id,
            team_id: &team_id,
        })
        .await?;

    let Some(member_info) = member_info else {
        return Err(assignment_role_not_assignable_perm_error());
    };

    if !member_info.roles.contains_mask(roles) {
        return Err(assignment_role_not_assignable_perm_error());
    }

    Ok(())
}

/// Resolve the owning team ID from a chapter ID via its comic and workset.
async fn resolve_team_id<P>(
    proxy: &mut P,
    chapter_id: &str,
) -> RegularResult<String>
where
    P: for<'a, 'b> Proxy<GetChapterInfo<'a, 'b>, Error = RegularError>
        + for<'a, 'b> Proxy<GetComicInfo<'a, 'b>, Error = RegularError>
        + for<'a> Proxy<GetWorksetInfo<'a>, Error = RegularError>,
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

    Ok(workset_info.team_id)
}

/// Construct a generic "assignment list forbidden" permission error.
fn assignment_list_permission_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-forbidden"),
    }
}

/// Construct a "chapter admin required" permission error.
fn chapter_admin_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-admin-required"),
    }
}

/// Construct a "assignment self-reduce forbidden" permission error.
fn assignment_self_reduce_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-forbidden"),
    }
}

/// Construct an "admin role cannot be assigned through this flow" args error.
fn assignment_role_not_assignable_args_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-chapter-role-not-assignable"),
    }
}

/// Construct a "role not assignable because member lacks permission" error.
fn assignment_role_not_assignable_perm_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-role-not-assignable"),
    }
}
