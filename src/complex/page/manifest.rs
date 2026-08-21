//! Pure matching for authoritative chapter page manifests.

#[cfg(test)]
mod tests;

use std::cmp::Ordering;
use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::model::read::proj::page::PageInfo;
use crate::model::write::page::PageImageSpec;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// One manifest position and the old page assigned to it, when any.
pub struct ManifestMatch {
    /// Index of the matched existing page in the input slice, or `None` for a new page.
    pub existing_index: Option<usize>,
}

/// Stable matching result for an authoritative page manifest.
pub struct ManifestPlan {
    //
    /// Ordered match results aligning each input page to an existing page or a new slot.
    pub matches: Vec<ManifestMatch>,
    /// Indexes of existing pages that were not matched and should be removed.
    pub deleted_existing_indexes: Vec<usize>,
}

/// Pure operations for authoritative chapter page manifests.
pub struct PageManifestComplex;

impl PageManifestComplex {
    /// Matches explicit identities first, then consumes automatic hash cands.
    pub fn build(
        chapter_id: &str,
        existing_page_infos: &[PageInfo],
        page_specs: &[PageImageSpec],
    ) -> BaseRest<ManifestPlan> {
        //
        let (mut assigned_existing_indexes, mut consumed_existing_indexes) =
            (vec![None; page_specs.len()], HashSet::new());

        for (request_index, page_spec) in page_specs.iter().enumerate() {
            //
            let Some(page_id) = &page_spec.page_id else {
                continue;
            };

            let existing_index = existing_page_infos
                .iter()
                .position(|page_info| {
                    //
                    page_info.id == *page_id
                        && page_info.chapter_id == chapter_id
                })
                .ok_or_else(|| {
                    //
                    let err_message = trl("error-page-not-found");

                    tracing::warn!(
                        err_variant = ?ExpectedVariant::Args,
                        err_message = %err_message,
                        chapter_id = %chapter_id,
                        page_id = %page_id,
                        request_index = request_index,
                        existing_page_count = existing_page_infos.len(),
                        "expected error: manifest page not found",
                    );

                    BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: err_message,
                    }
                })?;

            consumed_existing_indexes.insert(existing_index);

            assigned_existing_indexes[request_index] = Some(existing_index);
        }

        for (request_index, page_spec) in page_specs.iter().enumerate() {
            //
            if page_spec.page_id.is_some() {
                continue;
            }

            let existing_index = existing_page_infos
                .iter()
                .enumerate()
                .filter(|(existing_index, page_info)| {
                    //
                    !consumed_existing_indexes.contains(existing_index)
                        && page_info.image_hash.as_ref()
                            == Some(&page_spec.image_hash)
                        && page_info.image_ext == Some(page_spec.ext)
                })
                .min_by(|(_, left), (_, right)| cand_order(left, right))
                .map(|(existing_index, _)| existing_index);

            let Some(existing_index) = existing_index else {
                continue;
            };

            consumed_existing_indexes.insert(existing_index);

            assigned_existing_indexes[request_index] = Some(existing_index);
        }

        let matches = assigned_existing_indexes
            .into_iter()
            .map(|existing_index| ManifestMatch { existing_index })
            .collect();

        let deleted_existing_indexes = (0..existing_page_infos.len())
            .filter(|existing_index| {
                !consumed_existing_indexes.contains(existing_index)
            })
            .collect();

        accept(ManifestPlan {
            matches,
            deleted_existing_indexes,
        })
    }
}

// Compare two cands by translated state, upload time, and index for stable matching.
fn cand_order(left: &PageInfo, right: &PageInfo) -> Ordering {
    //
    right
        .total_unit_count
        .gt(&0)
        .cmp(&left.total_unit_count.gt(&0))
        .then_with(|| right.is_image_uploaded.cmp(&left.is_image_uploaded))
        .then_with(|| left.index.cmp(&right.index))
        .then_with(|| left.id.cmp(&right.id))
}
