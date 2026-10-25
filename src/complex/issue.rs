//! Pure validation for complete chapter review imports.

/// Review import permission rules.
pub mod perm;

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::model::read::proj::page_artwork::PageArtworkInfo;
use crate::model::shared::issue::IssueRect;
use crate::model::write::issue_import::PageIssueImport;
use crate::result::{BaseError, BaseRest, ExpectedVariant};

/// Constructs one localized issue validation or permission error.
pub fn error(variant: ExpectedVariant, key: &str) -> BaseError {
    //
    expected_error! {
        variant,
        message: trl(key);
        "review issue rejected",
    }
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
        return accept!(());
    }

    Err(error(ExpectedVariant::Args, "error-issue-rect"))
}

/// Validates every composite target and issue before replacement starts.
pub fn ensure_import_issue(
    pages: &[PageIssueImport],
    page_artwork_infos: &[PageArtworkInfo],
) -> BaseRest<()> {
    //
    let mut ids = HashSet::new();

    for page in pages {
        //
        if !ids.insert(&page.page_artwork_id)
            || !page_artwork_infos
                .iter()
                .any(|info| info.id == page.page_artwork_id)
        {
            return Err(error(
                ExpectedVariant::Args,
                "error-page-artwork-target",
            ));
        }

        //
        for issue in &page.issues {
            //
            if issue.trimmed_variant().is_empty() {
                return Err(error(ExpectedVariant::Args, "error-issue-input"));
            }

            if let Some(rect) = issue.rect {
                ensure_rect(rect)?;
            }
        }
    }

    accept!(())
}
