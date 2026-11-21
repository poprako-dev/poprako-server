use poprako_util::i18n::trl;

use crate::complex::util::{
    check_user_is_chapter_assignee,
    check_user_is_chapter_translator_or_proofreader, check_user_is_team_member,
};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::value::chapter_port::ChapterTranslationImportTarget;
use crate::value::role::RoleField;

/// Concrete evidence that grants chapter export access.
pub enum ChapterExportAccess<'a> {
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

/// Verify the caller may export chapter translations.
pub fn ensure_user_can_export_translation(
    access: &ChapterExportAccess<'_>,
) -> BaseRest<()> {
    //
    let access_check = match access {
        //
        ChapterExportAccess::Member { member_info } => {
            check_user_is_team_member(member_info)
        }

        ChapterExportAccess::Assignee { assignment_info } => {
            check_user_is_chapter_assignee(assignment_info)
        }
    };

    match access_check {
        //
        Ok(()) => accept!(()),

        Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            ..
        }) => {
            //
            let error_message = trl("error-chapter-port-export-perm-required");

            // The permission helper already recorded this failure.
            Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                message: error_message,
            })
        }

        Err(e) => Err(e),
    }
}

/// Verify the caller may import the selected chapter translation target.
pub fn ensure_user_can_import_translation(
    assignment_info: &AssignmentInfo,
    target: ChapterTranslationImportTarget,
) -> BaseRest<()> {
    //
    match check_user_is_chapter_translator_or_proofreader(assignment_info) {
        //
        Ok(()) => {}

        Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            ..
        }) => {
            //
            let error_message = trl("error-chapter-port-import-perm-required");

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                message: error_message,
            });
        }

        Err(e) => return Err(e),
    }

    let allowed = match target {
        //
        ChapterTranslationImportTarget::Translation => {
            assignment_info.roles.has_any_role(&[RoleField::TRANSLATOR])
        }

        ChapterTranslationImportTarget::Proofread => assignment_info
            .roles
            .has_any_role(&[RoleField::PROOFREADER]),

        ChapterTranslationImportTarget::Both => assignment_info
            .roles
            .has_every_role(&[RoleField::TRANSLATOR, RoleField::PROOFREADER]),
    };

    if allowed {
        return accept!(());
    }

    Err(expected_error! {
        ExpectedVariant::Perm,
        message: trl("error-chapter-port-import-target-perm-required");
        target = ?target,
        chapter_id = %assignment_info.chapter_id,
        user_id = %assignment_info.user_id,
        "chapter import target permission required",
    })
}
