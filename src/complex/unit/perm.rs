//! perm gates for Unit reads and edit fields.

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_chapter_assignee, check_user_is_team_member,
};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::write::unit::UnitEdit;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::unit::{UnitEditPerm, UnitTextPart};

/// Concrete evidence that grants Unit list access.
pub enum UnitListAccess<'a> {
    //
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

/// perm gates for Unit reads and edits.
pub struct UnitPermComplex;

impl UnitPermComplex {
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

        let err_message = trl("error-unit-transform-perm-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            perm = ?perm,
            part = ?part,
            operation = "transform",
            "expected error: unit transform perm required",
        );

        Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        })
    }

    /// Verifies that the caller may list Units on a Chapter Page.
    pub fn ensure_user_can_list_infos(
        access: UnitListAccess<'_>,
    ) -> BaseRest<()> {
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
                let err_message = trl("error-unit-list-perm-required");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Perm,
                    err_message = %err_message,
                    "expected error: unit list perm required",
                );

                Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: err_message,
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
            let err_message = trl("error-unit-edit-perm-required");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Perm,
                err_message = %err_message,
                perm = ?perm,
                operation = "edit_fields",
                "expected error: unit edit perm required",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                message: err_message,
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
                        let err_message = trl("error-unit-edit-perm-required");

                        tracing::warn!(
                            err_variant = ?ExpectedVariant::Perm,
                            err_message = %err_message,
                            perm = ?perm,
                            field = "translation",
                            operation = "create",
                            "expected error: unit translation perm required",
                        );

                        return Err(BaseError::Expected {
                            variant: ExpectedVariant::Perm,
                            message: err_message,
                        });
                    }

                    if revision.is_some() && !perm.can_proofread {
                        //
                        let err_message = trl("error-unit-edit-perm-required");

                        tracing::warn!(
                            err_variant = ?ExpectedVariant::Perm,
                            err_message = %err_message,
                            perm = ?perm,
                            field = "revision",
                            operation = "create",
                            "expected error: unit revision perm required",
                        );

                        return Err(BaseError::Expected {
                            variant: ExpectedVariant::Perm,
                            message: err_message,
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
                        let err_message = trl("error-unit-edit-perm-required");

                        tracing::warn!(
                            err_variant = ?ExpectedVariant::Perm,
                            err_message = %err_message,
                            perm = ?perm,
                            field = "translation",
                            operation = "save",
                            "expected error: unit translation perm required",
                        );

                        return Err(BaseError::Expected {
                            variant: ExpectedVariant::Perm,
                            message: err_message,
                        });
                    }

                    if !revision.is_skip() && !perm.can_proofread {
                        //
                        let err_message = trl("error-unit-edit-perm-required");

                        tracing::warn!(
                            err_variant = ?ExpectedVariant::Perm,
                            err_message = %err_message,
                            perm = ?perm,
                            field = "revision",
                            operation = "save",
                            "expected error: unit revision perm required",
                        );

                        return Err(BaseError::Expected {
                            variant: ExpectedVariant::Perm,
                            message: err_message,
                        });
                    }
                }

                UnitEdit::Delete { .. } => {}
            };
        }

        accept(())
    }
}
