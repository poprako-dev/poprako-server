//! Composite page write permissions.

use poprako_util::i18n::trl;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::role::RoleField;

/// Allows assigned typesetters, redrawers and reviewers to maintain composites.
pub fn ensure_user_can_write(assignment_info: &AssignmentInfo) -> BaseRest<()> {
    //
    if assignment_info.roles.has_any_role(&[
        RoleField::TYPESETTER,
        RoleField::REDRAWER,
        RoleField::REVIEWER,
    ]) {
        return accept(());
    }

    Err(BaseError::expected(
        ExpectedVariant::Perm,
        trl("error-page-artwork-write-role-required"),
    ))
}
