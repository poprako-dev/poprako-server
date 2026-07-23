//! Pure matching for authoritative chapter page manifests.

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::data::page::PageImageParams;
use crate::model::page::PageInfo;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};

#[cfg(test)]
mod tests;

/// One manifest position and the old page assigned to it, when any.
pub struct ManifestMatch {
    /// Index of the matched existing page in the input slice, or `None` for a new page.
    pub existing_index: Option<usize>,
}

/// Stable matching result for an authoritative page manifest.
pub struct ManifestPlan {
    /// Ordered match results aligning each input page to an existing page or a new slot.
    pub matches: Vec<ManifestMatch>,
    /// Indexes of existing pages that were not matched and should be removed.
    pub deleted_existing_indexes: Vec<usize>,
}

fn args_err(key: &str) -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl(key),
    }
}

fn validate_same_hash_metadata(
    page_info: &PageInfo,
    page_input: &PageImageParams,
) -> BaseResult<()> {
    //
    if page_info.image_hash == page_input.image_hash
        && (page_info.image_byte_length != page_input.byte_length
            || page_info.image_ext != page_input.ext)
    {
        return Err(args_err("error-invalid-page-image-identity"));
    }

    accept(())
}

fn candidate_order(left: &PageInfo, right: &PageInfo) -> std::cmp::Ordering {
    right
        .total_unit_count
        .gt(&0)
        .cmp(&left.total_unit_count.gt(&0))
        .then_with(|| right.image_uploaded.cmp(&left.image_uploaded))
        .then_with(|| left.index.cmp(&right.index))
        .then_with(|| left.id.cmp(&right.id))
}

/// Matches explicit identities first, then consumes automatic hash candidates.
pub fn build(
    chapter_id: &str,
    existing_page_infos: &[PageInfo],
    page_inputs: &[PageImageParams],
) -> BaseResult<ManifestPlan> {
    //
    let mut assigned_existing_indexes = vec![None; page_inputs.len()];

    let mut consumed_existing_indexes = HashSet::new();

    for (request_index, page_input) in page_inputs.iter().enumerate() {
        //
        let Some(page_id) = &page_input.page_id else {
            continue;
        };

        let existing_index = existing_page_infos
            .iter()
            .position(|page_info| {
                page_info.id == *page_id && page_info.chapter_id == chapter_id
            })
            .ok_or_else(|| args_err("error-page-not-found"))?;

        validate_same_hash_metadata(
            &existing_page_infos[existing_index],
            page_input,
        )?;

        consumed_existing_indexes.insert(existing_index);

        assigned_existing_indexes[request_index] = Some(existing_index);
    }

    for (request_index, page_input) in page_inputs.iter().enumerate() {
        //
        if page_input.page_id.is_some() {
            continue;
        }

        let existing_index = existing_page_infos
            .iter()
            .enumerate()
            .filter(|(existing_index, page_info)| {
                !consumed_existing_indexes.contains(existing_index)
                    && page_info.image_hash == page_input.image_hash
            })
            .min_by(|(_, left), (_, right)| candidate_order(left, right))
            .map(|(existing_index, _)| existing_index);

        let Some(existing_index) = existing_index else {
            continue;
        };

        validate_same_hash_metadata(
            &existing_page_infos[existing_index],
            page_input,
        )?;

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
