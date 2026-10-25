//! Pure matching for authoritative chapter page manifests.

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::model::write::page_source::PageSourceImageSpec;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::value::business_id::BusinessIdKind;
use crate::value::page_source::{
    ManifestMatch, ManifestPlan, PageSourceManifestCand,
};

/// Matches only explicit identities; absent identities create new source pages.
pub fn build(
    chapter_id: &str,
    cands: &[PageSourceManifestCand<'_>],
    page_source_specs: &[PageSourceImageSpec],
) -> BaseRest<ManifestPlan> {
    //
    let mut assigned_indexes = vec![None; page_source_specs.len()];

    let mut consumed_indexes = HashSet::new();

    for (request_index, (assigned_index, page_source_spec)) in assigned_indexes
        .iter_mut()
        .zip(page_source_specs)
        .enumerate()
    {
        let Some(page_source_id) = &page_source_spec.page_source_id else {
            continue;
        };

        let existing_index = cands
            .iter()
            .position(|cand| {
                //
                BusinessIdKind::PageSource.is_valid_id(page_source_id)
                    && cand.id == page_source_id
                    && cand.chapter_id == chapter_id
            })
            .ok_or_else(|| {
                page_source_not_found(chapter_id, page_source_id, request_index)
            })?;

        consumed_indexes.insert(existing_index);

        *assigned_index = Some(existing_index);
    }

    let matches = assigned_indexes
        .into_iter()
        .map(|existing_index| ManifestMatch { existing_index })
        .collect();

    let deleted_existing_indexes = (0..cands.len())
        .filter(|existing_index| !consumed_indexes.contains(existing_index))
        .collect();

    accept!(ManifestPlan {
        matches,
        deleted_existing_indexes,
    })
}

// Builds the expected missing-page error with matching diagnostics.
fn page_source_not_found(
    chapter_id: &str,
    page_source_id: &str,
    request_index: usize,
) -> BaseError {
    //
    expected_error! {
        ExpectedVariant::Args,
        message: trl("error-page-not-found");
        chapter_id,
        page_source_id,
        request_index,
        "manifest page not found",
    }
}
