//! Chapter page-count validation.

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::config::ImageConfig;
use crate::model::write::page::PageImageSpec;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::image::ImageKind;

/// Validates image metadata and page-count constraints before page reservation.
pub fn validate_page_specs(
    image_config: &ImageConfig,
    page_specs: &[PageImageSpec],
    chapter_id: &str,
    user_id: &str,
) -> BaseRest<i32> {
    //
    let page_count = i32::try_from(page_specs.len()).map_err(|_| {
        //
        let err_message = trl("error-invalid-page-count");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            chapter_id = %chapter_id,
            user_id = %user_id,
            page_count = page_specs.len(),
            "expected error: invalid page count",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        }
    })?;

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
pub fn validate_page_count(page_count: i32) -> BaseRest<()> {
    //
    if !(1..=200).contains(&page_count) {
        //
        let err_message = trl("error-invalid-page-count");

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
