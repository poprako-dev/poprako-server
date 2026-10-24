//! Pure rules for versioned annotation changes.

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::complex::issue as issue_complex;
use crate::model::read::proj::issue::IssueInfo;
use crate::model::read::proj::issue_edit::{
    IssueEditReviewVer, IssueEditVariantCount,
};
use crate::model::write::issue::IssueEntry;
use crate::result::{BaseError, BaseRest, ExpectedVariant};

/// Formats an opaque identity for this chapter's current review.
pub fn review_key(chapter_id: &str, ver: IssueEditReviewVer) -> String {
    format!("{}:{}", chapter_id, ver.gen_no)
}

/// Rejects stale page or chapter context without mutating it.
pub fn conflict() -> BaseError {
    //
    retryable_error! {
        message: trl("error-review-changed");
        "review context changed"
    }
}

/// Validates the whole replacement before persistence, including identity ownership.
pub fn ensure_entries(
    page_id: &str,
    entries: &[IssueEntry],
    existing: &[IssueInfo],
) -> BaseRest<()> {
    //
    let mut ids = HashSet::new();

    for entry in entries {
        //
        if entry.id.trim().is_empty()
            || !ids.insert(entry.id.as_str())
            || entry.trimmed_variant().is_empty()
        {
            return Err(issue_complex::error(
                ExpectedVariant::Args,
                "error-issue-input",
            ));
        }

        if let Some(rect) = entry.rect {
            issue_complex::ensure_rect(rect)?;
        }
    }

    if existing
        .iter()
        .any(|issue| issue.page_artwork_id != page_id)
    {
        return Err(issue_complex::error(
            ExpectedVariant::Args,
            "error-page-artwork-target",
        ));
    }

    accept!(())
}

/// Default names are fixed and rename never merges separate categories.
pub fn ensure_rename(
    from: &str,
    to: &str,
    counts: &[IssueEditVariantCount],
) -> BaseRest<()> {
    //
    let defaults = ["居中", "字体", "字号", "字重", "修图"];

    if from.is_empty()
        || to.is_empty()
        || defaults.contains(&from)
        || defaults.contains(&to)
    {
        return Err(issue_complex::error(
            ExpectedVariant::Args,
            "error-review-category-name",
        ));
    }

    if from != to && counts.iter().any(|item| item.variant == to) {
        //
        return Err(issue_complex::error(
            ExpectedVariant::Args,
            "error-review-category-name",
        ));
    }

    accept!(())
}
