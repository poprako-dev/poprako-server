//! Chapter page-count validation.

use std::collections::{HashMap, HashSet};

use poprako_util::i18n::{trl, trl_kv};

use crate::complex::{
    image as image_complex, page_source as page_source_complex,
};
use crate::config::image::ImageConfig;
use crate::model::write::page_source::PageSourceImageSpec;
use crate::result::{BaseRest, ExpectedVariant};
use crate::value::image::ImageKind;
use crate::value::page_source::MAX_CHAPTER_PAGE_SOURCE_COUNT;

/// Minimum number of pages accepted by a chapter manifest.
pub const MIN_CHAPTER_PAGE_SOURCE_COUNT: usize = 1;

/// Validates image metadata and page-count constraints before page allocation.
pub fn validate_page_source_specs(
    image_config: &ImageConfig,
    page_source_specs: &[PageSourceImageSpec],
    chapter_id: &str,
    user_id: &str,
) -> BaseRest<usize> {
    //
    let page_source_count = page_source_specs.len();

    validate_page_source_count(page_source_count)?;

    for page_source_spec in page_source_specs {
        //
        page_source_complex::ensure_raw_ident(
            page_source_spec.raw_ident.as_deref(),
        )?;
    }

    for new_byte_len in page_source_specs
        .iter()
        .filter_map(|page_source_spec| page_source_spec.new_byte_len)
    {
        image_complex::ensure_byte_length(
            image_config,
            new_byte_len,
            ImageKind::PageSourceImage,
        )?;
    }

    let mut explicit_page_source_ids = HashSet::new();

    for page_source_spec in page_source_specs {
        //
        let Some(page_source_id) = &page_source_spec.page_source_id else {
            continue;
        };

        if !explicit_page_source_ids.insert(page_source_id) {
            //
            return Err(expected_error! {
                ExpectedVariant::Args,
                message: trl("error-duplicate-page-id");
                chapter_id = %chapter_id,
                user_id = %user_id,
                page_source_id = %page_source_id,
                "duplicate page id in allocation",
            });
        }
    }

    accept!(page_source_count)
}

/// Validates the 200-page manifest cap, which bounds practical upload and review capacity.
pub fn validate_page_source_count(page_source_count: usize) -> BaseRest<()> {
    //
    if !(MIN_CHAPTER_PAGE_SOURCE_COUNT..=MAX_CHAPTER_PAGE_SOURCE_COUNT)
        .contains(&page_source_count)
    {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: invalid_page_source_count_message();
            page_source_count,
            "invalid page count",
        });
    }

    accept!(())
}

// Builds the translated page-count validation message.
fn invalid_page_source_count_message() -> String {
    //
    let args = HashMap::from([
        ("min_count".into(), MIN_CHAPTER_PAGE_SOURCE_COUNT.into()),
        ("max_count".into(), MAX_CHAPTER_PAGE_SOURCE_COUNT.into()),
    ]);

    trl_kv("error-invalid-page-count", &args)
}
