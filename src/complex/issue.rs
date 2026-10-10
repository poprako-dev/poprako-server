//! Pure validation for complete chapter review imports.

/// Review import permission rules.
pub mod perm;

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::data::instr::issue::ImportChapterIssuesInstr;
use crate::model::read::proj::page_artwork::PageArtworkInfo;
use crate::model::shared::issue::IssueRect;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Constructs one localized issue validation or permission error.
pub fn error(variant: ExpectedVariant, key: &str) -> BaseError {
    //
    let msg = trl(key);

    tracing::warn!(err_variant = ?variant, err_msg = %msg, "review issue rejected");

    BaseError::Expected { variant, msg }
}

/// Ensures finite, positive geometry entirely contained within the composite.
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

/// Validates every composite target and issue before replacement starts.
pub fn ensure_import_issue(
    instr: &ImportChapterIssuesInstr,
    page_artwork_infos: &[PageArtworkInfo],
) -> BaseRest<()> {
    //
    let mut ids = HashSet::new();

    for page_instr in &instr.pages {
        //
        if !ids.insert(&page_instr.page_artwork_id)
            || !page_artwork_infos
                .iter()
                .any(|info| info.id == page_instr.page_artwork_id)
        {
            return Err(error(
                ExpectedVariant::Args,
                "error-page-artwork-target",
            ));
        }

        //
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
