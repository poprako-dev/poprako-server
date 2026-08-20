//! Shared pure permission helpers for the complex layer.

use poprako_util::i18n::trl;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::role::{RoleField, RoleMask};

/// Verify that membership evidence exists.
pub fn check_user_is_team_member(_member_info: &MemberInfo) -> BaseRest<()> {
    accept(())
}

/// Verify that membership evidence contains a translator or proofreader role.
pub fn check_user_is_team_translator_or_proofreader(
    member_info: &MemberInfo,
) -> BaseRest<()> {
    //
    if !member_info
        .roles
        .has_any_role(&[RoleField::TRANSLATOR, RoleField::PROOFREADER])
    {
        //
        return reject_perm(
            "error-team-translator-or-proofreader-required",
            "team_translation_role_missing",
        );
    }

    accept(())
}

/// Verify that membership evidence contains the admin and required roles.
pub fn check_user_is_team_admin_with_roles(
    member_info: &MemberInfo,
    required_roles: Option<RoleMask>,
) -> BaseRest<()> {
    //
    if !member_info.roles.has_any_role(&[RoleField::ADMIN]) {
        //
        return reject_perm(
            "error-team-admin-required",
            "team_admin_role_missing",
        );
    }

    if required_roles
        .is_some_and(|roles| !member_info.roles.contains_mask(roles))
    {
        return reject_perm(
            "error-chapter-role-not-assignable",
            "team_required_roles_missing",
        );
    }

    accept(())
}

/// Verify that membership evidence contains the admin role.
pub fn check_user_is_team_admin(member_info: &MemberInfo) -> BaseRest<()> {
    check_user_is_team_admin_with_roles(member_info, None)
}

/// Verify that assignment evidence exists.
pub fn check_user_is_chapter_assignee(
    _assignment_info: &AssignmentInfo,
) -> BaseRest<()> {
    accept(())
}

/// Verify that assignment evidence contains a translator or proofreader role.
pub fn check_user_is_chapter_translator_or_proofreader(
    assignment_info: &AssignmentInfo,
) -> BaseRest<()> {
    //
    if !assignment_info
        .roles
        .has_any_role(&[RoleField::TRANSLATOR, RoleField::PROOFREADER])
    {
        return reject_perm(
            "error-chapter-translator-or-proofreader-required",
            "chapter_translation_role_missing",
        );
    }

    accept(())
}

// Build and log one expected permission error.
fn reject_perm(message_key: &str, event: &'static str) -> BaseRest<()> {
    //
    let err_message = trl(message_key);

    tracing::warn!(
        err_variant = ?ExpectedVariant::Perm,
        err_message = %err_message,
        event,
        "expected permission error",
    );

    Err(BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: err_message,
    })
}
