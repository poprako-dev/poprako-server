//! Chapter page-count validation.

use std::collections::{HashMap, HashSet};

use poprako_util::i18n::{trl, trl_kv};

use crate::complex::image::ImageComplex;
use crate::config::ImageConfig;
use crate::model::read::proj::page::PageInfo;
use crate::model::write::page::PageImageSpec;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::image::ImageKind;

/// Minimum number of pages accepted by a chapter manifest.
pub const MIN_CHAPTER_PAGE_COUNT: usize = 1;

/// Maximum number of pages accepted by a chapter manifest.
pub const MAX_CHAPTER_PAGE_COUNT: usize = 200;

/// Validates image metadata and page-count constraints before page reservation.
pub fn validate_page_specs(
    image_config: &ImageConfig,
    page_specs: &[PageImageSpec],
    chapter_id: &str,
    user_id: &str,
) -> BaseRest<usize> {
    //
    let page_count = page_specs.len();

    validate_page_count(page_count)?;

    for new_byte_len in page_specs
        .iter()
        .filter_map(|page_spec| page_spec.new_byte_len)
    {
        ImageComplex::ensure_byte_length(
            image_config,
            new_byte_len,
            ImageKind::PageImage,
        )?;
    }

    let mut explicit_page_ids = HashSet::new();

    for page_spec in page_specs {
        //
        let Some(page_id) = &page_spec.page_id else {
            continue;
        };

        if !explicit_page_ids.insert(page_id) {
            //
            let err_message = trl("error-duplicate-page-id");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                chapter_id = %chapter_id,
                user_id = %user_id,
                page_id = %page_id,
                "expected error: duplicate page id in reservation",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }
    }

    accept(page_count)
}

/// Validates the 200-page manifest cap, which bounds practical upload and review capacity.
pub fn validate_page_count(page_count: usize) -> BaseRest<()> {
    //
    if !(MIN_CHAPTER_PAGE_COUNT..=MAX_CHAPTER_PAGE_COUNT).contains(&page_count)
    {
        //
        let err_message = invalid_page_count_message();

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            page_count,
            "expected error: invalid page count",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    accept(())
}

/// Builds the unrecoverable error for an invalid manifest index.
pub fn invalid_manifest_index(
    chapter_id: &str,
    user_id: &str,
    existing_index: usize,
    existing_page_count: usize,
) -> BaseError {
    //
    tracing::error!(
        err_message = %"page manifest referenced an invalid existing page index",
        chapter_id,
        user_id,
        existing_index,
        existing_page_count,
        "internal invariant violated: invalid page manifest index",
    );

    BaseError::Unrecoverable {
        message: "page manifest referenced an invalid existing page index"
            .into(),
    }
}

/// Converts a validated page position for the response contract.
pub fn checked_page_index(index: usize) -> BaseRest<u32> {
    //
    u32::try_from(index).map_err(|_| BaseError::Unrecoverable {
        message: "[reserve_chapter_pages] page index must be non-negative"
            .into(),
    })
}

/// Computes the image version for a retained or replaced image.
pub fn next_image_version(
    page_info: &PageInfo,
    identity_changed: bool,
) -> BaseRest<u32> {
    //
    if identity_changed {
        //
        return page_info
            .image_version
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| BaseError::Unrecoverable {
                message: "[reserve_chapter_pages] image version overflow"
                    .into(),
            });
    }

    page_info
        .image_version
        .ok_or_else(|| BaseError::Unrecoverable {
            message:
                "[reserve_chapter_pages] retained page image version is missing"
                    .into(),
        })
}

// Builds the translated page-count validation message.
fn invalid_page_count_message() -> String {
    //
    let args = HashMap::from([
        ("min_count".into(), MIN_CHAPTER_PAGE_COUNT.into()),
        ("max_count".into(), MAX_CHAPTER_PAGE_COUNT.into()),
    ]);

    trl_kv("error-invalid-page-count", &args)
}
