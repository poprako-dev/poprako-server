use poprako_util::i18n::trl;

use crate::complex::chapter::role::{check_join_role, check_workflow_role};
use crate::complex::util::{
    check_user_is_team_admin, check_user_is_team_admin_with_roles,
    check_user_is_team_member,
};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::chapter::stage::{Stage, StageOper};
use crate::value::role::{RoleField, RoleMask};

// Verify that assignment evidence contains the chapter-admin role.
fn check_admin(assignment_info: &AssignmentInfo) -> BaseRest<()> {
    //
    if !assignment_info.roles.has_any_role(&[RoleField::ADMIN]) {
        //
        let err_message = trl("error-chapter-admin-required");

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}

/// Pure permission rules for chapter entities.
pub struct ChapterPermComplex;

impl ChapterPermComplex {
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
        check_user_is_team_admin_with_roles(
            member_info,
            preset_assignment_roles,
        )
    }

    /// Verify the caller may update chapter metadata.
    pub fn ensure_user_can_update_info(
        assignment_info: &AssignmentInfo,
    ) -> BaseRest<()> {
        check_admin(assignment_info)
    }

    /// Verify the caller may pin a chapter.
    pub fn ensure_user_can_mark_pinned(
        assignment_info: &AssignmentInfo,
    ) -> BaseRest<()> {
        check_admin(assignment_info)
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
}
