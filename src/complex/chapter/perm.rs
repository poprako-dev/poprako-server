use crate::complex::assignment::perm as assignment_perm_complex;
use crate::complex::chapter::role::{check_join_role, check_workflow_role};
use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_member,
};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::result::{BaseRest, accept};
use crate::value::chapter::stage::{Stage, StageOper};
use crate::value::role::RoleMask;

/// Verify the caller may list chapters.
pub const fn ensure_user_can_list_infos(
    member_info: &MemberInfo,
) -> BaseRest<()> {
    check_user_is_team_member(member_info)
}

/// Verify the caller may read a chapter.
pub const fn ensure_user_can_get_info(
    member_info: &MemberInfo,
) -> BaseRest<()> {
    check_user_is_team_member(member_info)
}

/// Verify the caller may read a pinned chapter.
pub const fn ensure_user_can_get_pinned(
    member_info: &MemberInfo,
) -> BaseRest<()> {
    check_user_is_team_member(member_info)
}

/// Verify the caller may create a chapter.
pub fn ensure_user_can_create(
    member_info: &MemberInfo,
    preset_assignment_roles: Option<RoleMask>,
) -> BaseRest<()> {
    //
    check_user_is_team_admin(member_info)?;

    if let Some(roles) = preset_assignment_roles {
        //
        assignment_perm_complex::ensure_user_can_take_roles(
            member_info,
            roles,
        )?;
    }

    accept(())
}

/// Verify the caller administers the owning team.
pub fn ensure_user_can_manage(member_info: &MemberInfo) -> BaseRest<()> {
    check_user_is_team_admin(member_info)
}

/// Verify the caller may update chapter metadata.
pub fn ensure_user_can_update_info(member_info: &MemberInfo) -> BaseRest<()> {
    check_user_is_team_admin(member_info)
}

/// Verify the caller may pin a chapter.
pub fn ensure_user_can_mark_pinned(member_info: &MemberInfo) -> BaseRest<()> {
    check_user_is_team_admin(member_info)
}

/// Verify the caller may apply a workflow operation.
pub fn ensure_user_can_update_stage(
    assignment_info: &AssignmentInfo,
    assignment_infos: &[AssignmentInfo],
    stage: Stage,
    oper: StageOper,
) -> BaseRest<()> {
    check_workflow_role(assignment_info, assignment_infos, stage, oper)
}

/// Verify the caller may join a chapter.
pub fn ensure_user_can_join(
    member_info: &MemberInfo,
    roles: RoleMask,
) -> BaseRest<()> {
    check_join_role(member_info, roles)
}

/// Verify the caller may delete a chapter.
pub fn ensure_user_can_delete(member_info: &MemberInfo) -> BaseRest<()> {
    check_user_is_team_admin(member_info)
}
