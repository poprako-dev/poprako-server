//! Pure matching for authoritative chapter page manifests.

#[cfg(test)]
mod tests;

use std::cmp::Ordering;
use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::model::write::page::PageImageSpec;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Business fields used to match an existing page to an incoming image.
pub struct PageManifestCand<'a> {
    //
    /// Stable page identity.
    pub id: &'a str,

    /// Owning chapter identity.
    pub chapter_id: &'a str,
    /// Current page position.
    pub index: usize,

    /// Whether translation data already exists on the page.
    pub has_units: bool,

    /// Whether the current image is available to readers.
    pub image_uploaded: bool,
    /// Current image hash, when an object is attached.
    pub image_hash: Option<&'a [u8]>,
    /// Current image extension, when an object is attached.
    pub image_ext: Option<&'a str>,
}

/// One manifest position and the old page assigned to it, when any.
pub struct ManifestMatch {
    /// Index of the matched existing page, or none for a new page.
    pub existing_index: Option<usize>,
}

/// Stable matching result for an authoritative page manifest.
pub struct ManifestPlan {
    //
    /// Ordered match results aligned with the requested manifest.
    pub matches: Vec<ManifestMatch>,
    /// Existing page indexes not consumed by the requested manifest.
    pub deleted_existing_indexes: Vec<usize>,
}

/// Pure operations for authoritative chapter page manifests.
pub struct PageManifestComplex;

impl PageManifestComplex {
    /// Reserves explicit identities before consuming automatic hash matches.
    pub fn build(
        chapter_id: &str,
        cands: &[PageManifestCand<'_>],
        page_specs: &[PageImageSpec],
    ) -> BaseRest<ManifestPlan> {
        //
        let mut assigned_indexes = vec![None; page_specs.len()];

        let mut consumed_indexes = HashSet::new();

        for (request_index, (assigned_index, page_spec)) in
            assigned_indexes.iter_mut().zip(page_specs).enumerate()
        {
            let Some(page_id) = &page_spec.page_id else {
                continue;
            };

            let existing_index = cands
                .iter()
                .position(|cand| {
                    cand.id == page_id && cand.chapter_id == chapter_id
                })
                .ok_or_else(|| {
                    page_not_found(chapter_id, page_id, request_index)
                })?;

            consumed_indexes.insert(existing_index);

            *assigned_index = Some(existing_index);
        }

        for (assigned_index, page_spec) in
            assigned_indexes.iter_mut().zip(page_specs)
        {
            let None = page_spec.page_id else {
                continue;
            };

            let existing_index = cands
                .iter()
                .enumerate()
                .filter(|(existing_index, cand)| {
                    //
                    !consumed_indexes.contains(existing_index)
                        && cand.image_hash
                            == Some(page_spec.image_hash.as_bytes().as_slice())
                        && cand.image_ext == Some(page_spec.ext.suffix())
                })
                .min_by(|(_, left), (_, right)| cand_order(left, right))
                .map(|(existing_index, _)| existing_index);

            let Some(existing_index) = existing_index else {
                continue;
            };

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

        accept(ManifestPlan {
            matches,
            deleted_existing_indexes,
        })
    }
}

// Orders automatic cands by content preservation and stable identity.
fn cand_order(
    left: &PageManifestCand<'_>,
    right: &PageManifestCand<'_>,
) -> Ordering {
    //
    right
        .has_units
        .cmp(&left.has_units)
        .then_with(|| right.image_uploaded.cmp(&left.image_uploaded))
        .then_with(|| left.index.cmp(&right.index))
        .then_with(|| left.id.cmp(right.id))
}

// Builds the expected missing-page error with matching diagnostics.
fn page_not_found(
    chapter_id: &str,
    page_id: &str,
    request_index: usize,
) -> BaseError {
    //
    let err_message = trl("error-page-not-found");

    tracing::warn!(
        err_variant = ?ExpectedVariant::Args,
        err_message = %err_message,
        chapter_id,
        page_id,
        request_index,
        "expected error: manifest page not found",
    );

    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: err_message,
    }
}
