//! Pure rules for chapter assignments.

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_chapter_assignee, check_user_is_team_member,
};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::user::UserInfo;
use crate::model::write::assignment::AssignmentRoleRepl;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::role::{RoleField, RoleMask};

/// Pure domain operations for chapter assignments.
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

        preset_roles.map_or(admin_roles, |roles| roles.union(admin_roles))
    }

    /// Merge new roles into an existing assignment.
    pub fn merge_roles(
        assignment_info: &AssignmentInfo,
        roles: RoleMask,
    ) -> AssignmentRoleRepl {
        //
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
        //
        current_user_id == assignment_info.user_id
            && assignment_info.roles.has_any_role(&[RoleField::ADMIN])
            && !roles.has_any_role(&[RoleField::ADMIN])
    }

    /// Checks whether a chapter still has an admin after a role update.
    pub fn chapter_has_admin_after_role_update(
        assignment_infos: &[AssignmentInfo],
        user_id: &str,
        roles: RoleMask,
    ) -> bool {
        //
        assignment_infos.iter().any(|assignment_info| {
            //
            if assignment_info.user_id == user_id {
                roles.has_any_role(&[RoleField::ADMIN])
            } else {
                assignment_info.roles.has_any_role(&[RoleField::ADMIN])
            }
        })
    }
}

/// Evidence that grants access to a chapter assignment list.
pub enum AssignmentListAccess<'a> {
    //
    /// Access through team membership.
    Member {
        /// Team membership used to establish access.
        member_info: &'a MemberInfo,
    },

    /// Access through an assignment on the chapter.
    Assignee {
        /// Chapter assignment used to establish access.
        assignment_info: &'a AssignmentInfo,
    },
}

/// Evidence that grants access to a user assignment list.
pub enum UserAssignmentListAccess<'a> {
    //
    /// The caller owns the requested list.
    Owner,

    /// The caller is a super-admin.
    SuperAdmin {
        /// User projection used to verify super-admin status.
        user_info: &'a UserInfo,
    },
}

/// Evidence that grants an assignment-role update.
pub enum AssignmentRoleUpdateAccess<'a> {
    //
    /// The caller is a chapter admin.
    Admin {
        /// Chapter assignment used to verify admin status.
        assignment_info: &'a AssignmentInfo,
    },

    /// The caller is reducing their own existing roles.
    SelfReduce {
        /// Existing assignment used to constrain the self-update.
        assignment_info: &'a AssignmentInfo,
    },
}

/// Evidence that grants assignment deletion.
pub enum AssignmentDeleteAccess<'a> {
    //
    /// The caller owns the assignment.
    Owner,

    /// The caller is a chapter admin.
    Admin {
        /// Chapter assignment used to verify admin status.
        assignment_info: &'a AssignmentInfo,
    },
}

/// Pure permission rules for chapter assignments.
pub struct AssignmentPermComplex;

impl AssignmentPermComplex {
    /// Verify chapter assignment lists using membership or assignment evidence.
    pub const fn ensure_user_can_list_chapter_infos(
        access: &AssignmentListAccess<'_>,
    ) -> BaseRest<()> {
        //
        match access {
            //
            AssignmentListAccess::Member { member_info } => {
                check_user_is_team_member(member_info)
            }

            AssignmentListAccess::Assignee { assignment_info } => {
                check_user_is_chapter_assignee(assignment_info)
            }
        }
    }

    /// Verify user assignment lists using ownership or super-admin evidence.
    pub fn ensure_user_can_list_user_infos(
        access: &UserAssignmentListAccess<'_>,
    ) -> BaseRest<()> {
        //
        match access {
            //
            UserAssignmentListAccess::Owner => accept(()),

            UserAssignmentListAccess::SuperAdmin { user_info }
                if user_info.is_sadmin =>
            {
                accept(())
            }

            UserAssignmentListAccess::SuperAdmin { .. } => reject(
                ExpectedVariant::Perm,
                "error-forbidden",
                "assignment_list_permission_denied",
            ),
        }
    }

    /// Verify the caller may mutate assignment roles.
    pub fn ensure_user_can_update_roles(
        access: &AssignmentRoleUpdateAccess<'_>,
        subject_member_info: &MemberInfo,
        roles: RoleMask,
    ) -> BaseRest<()> {
        //
        match access {
            //
            AssignmentRoleUpdateAccess::Admin { assignment_info } => {
                check_admin(assignment_info)?;
            }

            AssignmentRoleUpdateAccess::SelfReduce { assignment_info } => {
                //
                if assignment_info.user_id != subject_member_info.user_id {
                    //
                    return reject(
                        ExpectedVariant::Perm,
                        "error-forbidden",
                        "assignment_self_reduce_target_mismatch",
                    );
                }

                if !assignment_info.roles.contains_mask(roles) {
                    //
                    return reject(
                        ExpectedVariant::Perm,
                        "error-forbidden",
                        "assignment_self_reduce_roles_not_held",
                    );
                }
            }
        }

        check_target_roles(subject_member_info, roles)
    }

    /// Verify the caller may delete the target assignment.
    pub fn ensure_user_can_delete(
        access: &AssignmentDeleteAccess<'_>,
    ) -> BaseRest<()> {
        //
        match access {
            //
            AssignmentDeleteAccess::Owner => accept(()),

            AssignmentDeleteAccess::Admin { assignment_info } => {
                check_admin(assignment_info)
            }
        }
    }

    /// Verify the target user may take the requested roles.
    pub fn ensure_user_can_take_roles(
        member_info: &MemberInfo,
        roles: RoleMask,
    ) -> BaseRest<()> {
        check_target_roles(member_info, roles)
    }
}

// Build and log one expected assignment permission error.
fn reject(
    variant: ExpectedVariant,
    message_key: &str,
    event: &'static str,
) -> BaseRest<()> {
    //
    let err_message = trl(message_key);

    tracing::warn!(
        err_variant = ?variant,
        err_message = %err_message,
        event,
        "expected assignment permission error",
    );

    Err(BaseError::Expected {
        variant,
        message: err_message,
    })
}

// Verify that assignment evidence contains the chapter-admin role.
fn check_admin(assignment_info: &AssignmentInfo) -> BaseRest<()> {
    //
    if !assignment_info.roles.has_any_role(&[RoleField::ADMIN]) {
        //
        return reject(
            ExpectedVariant::Perm,
            "error-chapter-admin-required",
            "chapter_admin_role_missing",
        );
    }

    accept(())
}

// Verify that membership evidence permits all requested assignment roles.
fn check_target_roles(
    member_info: &MemberInfo,
    roles: RoleMask,
) -> BaseRest<()> {
    //
    if roles.has_any_role(&[RoleField::ADMIN]) {
        //
        return reject(
            ExpectedVariant::Args,
            "error-chapter-role-not-assignable",
            "chapter_admin_role_not_assignable",
        );
    }

    if !member_info.roles.contains_mask(roles) {
        //
        return reject(
            ExpectedVariant::Perm,
            "error-chapter-role-not-assignable",
            "chapter_target_roles_missing",
        );
    }

    accept(())
}
