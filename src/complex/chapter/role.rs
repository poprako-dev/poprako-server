use poprako_util::i18n::trl;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::chapter::{Stage, StageOper};
use crate::value::role::{RoleField, RoleMask};

/// Verify the caller holds the workflow role required for a transition.
pub fn check_workflow_role(
    assignment_info: &AssignmentInfo,
    assignment_infos: &[AssignmentInfo],
    stage: Stage,
    oper: StageOper,
) -> BaseRest<()> {
    //
    let required_roles = required_roles_for_transition(stage, oper);

    if oper == StageOper::Advance {
        //
        let has_holder = assignment_infos
            .iter()
            .any(|info| info.roles.has_any_role(required_roles));

        if !has_holder {
            //
            return reject(
                "error-chapter-no-role-holder",
                "workflow_role_holder_missing",
            );
        }
    }

    if assignment_info.roles.has_any_role(&[RoleField::ADMIN]) {
        return accept(());
    }

    if required_roles.is_empty()
        || !assignment_info.roles.has_any_role(required_roles)
    {
        return reject(
            "error-chapter-workflow-role-required",
            "workflow_role_missing",
        );
    }

    accept(())
}

/// Verify the caller may join a chapter with the requested roles.
pub fn check_join_role(
    member_info: &MemberInfo,
    roles: RoleMask,
) -> BaseRest<()> {
    //
    if roles.has_any_role(&[RoleField::ADMIN]) {
        //
        let err_message = trl("error-chapter-role-not-assignable");

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    if !member_info.roles.contains_mask(roles) {
        //
        return reject(
            "error-chapter-role-not-assignable",
            "chapter_member_roles_missing",
        );
    }

    accept(())
}

// Resolve the assignment roles that can apply one workflow transition.
fn required_roles_for_transition(
    stage: Stage,
    oper: StageOper,
) -> &'static [RoleField] {
    //
    match (stage, oper) {
        //
        (Stage::RawProvide, StageOper::Advance) => &[RoleField::RAW_PROVIDER],

        (Stage::Translate, StageOper::Advance) => &[RoleField::TRANSLATOR],

        (Stage::Translate, StageOper::Revert) => {
            &[RoleField::TRANSLATOR, RoleField::PROOFREADER]
        }

        (Stage::Proofread, StageOper::Advance | StageOper::Revert) => {
            &[RoleField::PROOFREADER]
        }

        (Stage::TypesetRedraw, StageOper::Advance | StageOper::Revert) => {
            &[RoleField::TYPESETTER, RoleField::REDRAWER]
        }

        (Stage::Review, StageOper::Advance | StageOper::Revert) => {
            &[RoleField::REVIEWER]
        }

        (Stage::Publish, StageOper::Advance) => &[RoleField::PUBLISHER],

        _ => &[],
    }
}

// Build and log one expected chapter-role permission error.
fn reject(message_key: &str, event: &'static str) -> BaseRest<()> {
    //
    let err_message = trl(message_key);

    tracing::warn!(
        err_variant = ?ExpectedVariant::Perm,
        err_message = %err_message,
        event,
        "expected chapter role error",
    );

    Err(BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: err_message,
    })
}
