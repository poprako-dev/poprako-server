//! Pure validation for complete chapter review imports.

use poprako_util::i18n::trl;

use crate::data::instr::issue::ImportChapterIssuesInstr;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::shared::issue::IssueRect;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::role::RoleField;

/// Constructs one localized issue validation or permission error.
pub fn error(variant: ExpectedVariant, key: &str) -> BaseError {
    //
    let msg = trl(key);

    tracing::warn!(err_variant = ?variant, err_msg = %msg, "review issue rejected");

    BaseError::Expected { variant, msg }
}

/// Ensures finite, positive geometry entirely contained within the Page.
pub fn ensure_rect(rect: IssueRect) -> BaseRest<()> {
    //
    if [rect.x_coord, rect.y_coord, rect.width, rect.height]
        .iter()
        .all(|value| value.is_finite())
        && rect.x_coord >= 0.0
        && rect.y_coord >= 0.0
        && rect.width > 0.0
        && rect.height > 0.0
        && rect.x_coord + rect.width <= 1.0
        && rect.y_coord + rect.height <= 1.0
    {
        return accept(());
    }

    Err(error(ExpectedVariant::Args, "error-issue-rect"))
}

/// Rejects imports without a current REVIEWER assignment.
pub fn ensure_user_can_import(
    assignment_info: &AssignmentInfo,
) -> BaseRest<()> {
    //
    if assignment_info.roles.has_any_role(&[RoleField::REVIEWER]) {
        return accept(());
    }

    Err(error(
        ExpectedVariant::Perm,
        "error-issue-reviewer-required",
    ))
}

/// Validates every Page and issue before replacement starts.
pub fn ensure_import(
    instr: &ImportChapterIssuesInstr,
    page_count: usize,
) -> BaseRest<()> {
    //
    if instr.pages.len() != page_count {
        return Err(error(ExpectedVariant::Args, "error-issue-page-count"));
    }

    for page_instr in &instr.pages {
        //
        if i32::try_from(page_instr.issues.len()).is_err() {
            return Err(error(ExpectedVariant::Args, "error-issue-input"));
        }

        for issue_instr in &page_instr.issues {
            //
            if issue_instr.variant.trim().is_empty()
                || issue_instr
                    .layer_path
                    .as_ref()
                    .is_some_and(|path| path.trim().is_empty())
            {
                return Err(error(ExpectedVariant::Args, "error-issue-input"));
            }

            if let Some(rect_instr) = &issue_instr.rect {
                //
                let rect = IssueRect {
                    x_coord: rect_instr.x_coord,
                    y_coord: rect_instr.y_coord,
                    width: rect_instr.width,
                    height: rect_instr.height,
                };

                ensure_rect(rect)?;
            }
        }
    }

    accept(())
}
