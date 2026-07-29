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
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};
use crate::util::Patch;
use crate::value::unit::UnitEditPerm;

/// Permission gates for Unit reads and edits.
pub struct UnitPermComplex;

impl UnitPermComplex {
    /// Verifies that the caller may list Units on a Chapter Page.
    pub async fn ensure_user_can_list_infos<P>(
        proxy: &mut P,
        user_id: &str,
        chapter_id: &str,
    ) -> BaseResult<()>
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
            }) => Err(unit_list_permission_err()),

            Err(error) => Err(error),
        }
    }

    /// Enforces role presence and field-level content permissions.
    pub fn ensure_user_can_edit_fields(
        perm: UnitEditPerm,
        edits: &[UnitEdit],
    ) -> BaseResult<()> {
        //
        if !perm.can_translate && !perm.can_proofread {
            return Err(unit_edit_permission_err());
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
                    check_optional_content_perm(
                        translation,
                        perm.can_translate,
                    )?;

                    check_optional_content_perm(revision, perm.can_proofread)?;
                }

                UnitEdit::Save {
                    translation,
                    revision,
                    ..
                } => {
                    //
                    check_content_perm(translation, perm.can_translate)?;

                    check_content_perm(revision, perm.can_proofread)?;
                }

                UnitEdit::Delete { .. } => {}
            };
        }

        accept(())
    }
}

// Return a permission error for unit edit operations.
fn unit_edit_permission_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-unit-edit-permission-required"),
    }
}

// Return a permission error for list operations.
fn unit_list_permission_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-unit-list-permission-required"),
    }
}

// Validate optional content is only provided when the caller is allowed.
fn check_optional_content_perm<T>(
    field: &Option<T>,
    allowed: bool,
) -> BaseResult<()> {
    //
    if field.is_some() && !allowed {
        return Err(unit_edit_permission_err());
    }

    accept(())
}

// Validate patch content is only assigned when the caller has permission.
fn check_content_perm<T>(field: &Patch<T>, allowed: bool) -> BaseResult<()> {
    //
    if !field.is_skip() && !allowed {
        return Err(unit_edit_permission_err());
    }

    accept(())
}
