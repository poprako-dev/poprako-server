//! perm gates for Unit reads and edit fields.

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_chapter_assignee, check_user_is_team_member,
};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::write::unit::UnitEdit;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::role::RoleField;
use crate::value::unit::{UnitEditPerm, UnitTextPart};

/// Concrete evidence that grants Unit list access.
pub enum UnitListAccess<'a> {
    /// Access through team membership.
    Member {
        /// Team membership used to establish access.
        member_info: &'a MemberInfo,
    },

    /// Access through a chapter assignment.
    Assignee {
        /// Chapter assignment used to establish access.
        assignment_info: &'a AssignmentInfo,
    },
}

/// Derives field edit permissions from an optional Chapter assignment.
pub fn edit_perm(assignment_info: Option<&AssignmentInfo>) -> UnitEditPerm {
    //
    let has_role = |role| {
        assignment_info.is_some_and(|info| info.roles.has_any_role(&[role]))
    };

    UnitEditPerm {
        can_translate: has_role(RoleField::TRANSLATOR),
        can_proofread: has_role(RoleField::PROOFREADER),
    }
}

/// Verifies that the caller may transform the selected Unit text part.
pub fn ensure_user_can_transform(
    perm: UnitEditPerm,
    part: UnitTextPart,
) -> BaseRest<()> {
    //
    let allowed = match part {
        //
        UnitTextPart::TranslatedText => perm.can_translate,

        UnitTextPart::ProofreadText => perm.can_proofread,
    };

    if allowed {
        return accept(());
    }

    Err(expected_error! {
        ExpectedVariant::Perm,
        message: trl("error-unit-transform-perm-required");
        perm = ?perm,
        part = ?part,
        operation = "transform",
        "unit transform perm required",
    })
}

/// Verifies that the caller may list Units on a Chapter Page.
pub fn ensure_user_can_list_infos(access: &UnitListAccess<'_>) -> BaseRest<()> {
    //
    let access_check = match access {
        //
        UnitListAccess::Member { member_info } => {
            check_user_is_team_member(member_info)
        }

        UnitListAccess::Assignee { assignment_info } => {
            check_user_is_chapter_assignee(assignment_info)
        }
    };

    match access_check {
        //
        Ok(()) => accept(()),

        Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            ..
        }) => {
            //
            let err_msg = trl("error-unit-list-perm-required");

            // The permission helper already recorded this failure.
            Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                msg: err_msg,
            })
        }

        Err(error) => Err(error),
    }
}

/// Enforces role presence and field-level content perms.
pub fn ensure_user_can_edit_fields(
    perm: UnitEditPerm,
    edits: &[UnitEdit],
) -> BaseRest<()> {
    //
    if !perm.can_translate && !perm.can_proofread {
        //
        return Err(expected_error! {
            ExpectedVariant::Perm,
            message: trl("error-unit-edit-perm-required");
            perm = ?perm,
            operation = "edit_fields",
            "unit edit perm required",
        });
    }

    for edit in edits {
        //
        match edit {
            //
            UnitEdit::Create {
                translation,
                revision,
                ..
            } => {
                //
                if translation.is_some() && !perm.can_translate {
                    //
                    return Err(expected_error! {
                        ExpectedVariant::Perm,
                        message: trl("error-unit-edit-perm-required");
                        perm = ?perm,
                        field = "translation",
                        operation = "create",
                        "unit translation perm required",
                    });
                }

                if revision.is_some() && !perm.can_proofread {
                    //
                    return Err(expected_error! {
                        ExpectedVariant::Perm,
                        message: trl("error-unit-edit-perm-required");
                        perm = ?perm,
                        field = "revision",
                        operation = "create",
                        "unit revision perm required",
                    });
                }
            }

            UnitEdit::Save {
                translation,
                revision,
                ..
            } => {
                //
                if !translation.is_skip() && !perm.can_translate {
                    //
                    return Err(expected_error! {
                        ExpectedVariant::Perm,
                        message: trl("error-unit-edit-perm-required");
                        perm = ?perm,
                        field = "translation",
                        operation = "save",
                        "unit translation perm required",
                    });
                }

                if !revision.is_skip() && !perm.can_proofread {
                    //
                    return Err(expected_error! {
                        ExpectedVariant::Perm,
                        message: trl("error-unit-edit-perm-required");
                        perm = ?perm,
                        field = "revision",
                        operation = "save",
                        "unit revision perm required",
                    });
                }
            }

            UnitEdit::Delete { .. } => {}
        }
    }

    accept(())
}
