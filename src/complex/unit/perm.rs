//! Permission gates for Unit reads and edit fields.

use poprako_orchestra::Proxy;

use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_chapter_assignee, check_user_is_team_member_by_chapter,
};
use crate::model::write::unit::UnitEdit;
use crate::part::repo::oper::assignment::FindAssignmentInfo;
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::unit::UnitEditPerm;

/// Permission gates for Unit reads and edits.
pub struct UnitPermComplex;

impl UnitPermComplex {
    /// Verifies that the caller may list Units on a Chapter Page.
    pub async fn ensure_user_can_list_infos<P>(
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
        let member_check =
            check_user_is_team_member_by_chapter(proxy, user_id, chapter_id)
                .await;

        if member_check.is_ok() {
            return accept(());
        }

        match check_user_is_chapter_assignee(proxy, user_id, chapter_id).await {
            //
            Ok(()) => accept(()),

            Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                ..
            }) => {
                //
                let err_message = trl("error-unit-list-permission-required");

                tracing::warn!(
                    error_variant = ?ExpectedVariant::Perm,
                    err_message = %err_message,
                    user_id = %user_id,
                    chapter_id = %chapter_id,
                    "expected error: unit list permission required",
                );

                Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: err_message,
                })
            }

            Err(error) => Err(error),
        }
    }

    /// Enforces role presence and field-level content permissions.
    pub fn ensure_user_can_edit_fields(
        perm: UnitEditPerm,
        edits: &[UnitEdit],
    ) -> BaseRest<()> {
        //
        if !perm.can_translate && !perm.can_proofread {
            //
            let err_message = trl("error-unit-edit-permission-required");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Perm,
                err_message = %err_message,
                permission = ?perm,
                operation = "edit_fields",
                "expected error: unit edit permission required",
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
                        let err_message =
                            trl("error-unit-edit-permission-required");

                        tracing::warn!(
                            error_variant = ?ExpectedVariant::Perm,
                            err_message = %err_message,
                            permission = ?perm,
                            field = "translation",
                            operation = "create",
                            "expected error: unit translation permission required",
                        );

                        return Err(BaseError::Expected {
                            variant: ExpectedVariant::Perm,
                            message: err_message,
                        });
                    }

                    if revision.is_some() && !perm.can_proofread {
                        //
                        let err_message =
                            trl("error-unit-edit-permission-required");

                        tracing::warn!(
                            error_variant = ?ExpectedVariant::Perm,
                            err_message = %err_message,
                            permission = ?perm,
                            field = "revision",
                            operation = "create",
                            "expected error: unit revision permission required",
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
                        let err_message =
                            trl("error-unit-edit-permission-required");

                        tracing::warn!(
                            error_variant = ?ExpectedVariant::Perm,
                            err_message = %err_message,
                            permission = ?perm,
                            field = "translation",
                            operation = "save",
                            "expected error: unit translation permission required",
                        );

                        return Err(BaseError::Expected {
                            variant: ExpectedVariant::Perm,
                            message: err_message,
                        });
                    }

                    if !revision.is_skip() && !perm.can_proofread {
                        //
                        let err_message =
                            trl("error-unit-edit-permission-required");

                        tracing::warn!(
                            error_variant = ?ExpectedVariant::Perm,
                            err_message = %err_message,
                            permission = ?perm,
                            field = "revision",
                            operation = "save",
                            "expected error: unit revision permission required",
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
