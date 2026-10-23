//! Review import permissions.

use poprako_util::i18n::trl;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::result::{BaseRest, ExpectedVariant, accept};
use crate::value::role::RoleField;

/// Requires a current REVIEWER assignment for Issue import.
pub fn ensure_user_can_import_issue(
    assignment_info: &AssignmentInfo,
) -> BaseRest<()> {
    //
    if assignment_info.roles.has_any_role(&[RoleField::REVIEWER]) {
        return accept(());
    }

    Err(expected_error! {
        ExpectedVariant::Perm,
        message: trl("error-issue-reviewer-required");
        "reviewer assignment required for issue import",
    })
}
