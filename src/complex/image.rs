//! Complex domain logic for image lifecycle tracking — deletion and integrity-check ID generation for asynchronous image processing.

#[cfg(test)]
mod tests;

use poprako_util::i18n::trl;

use crate::config::ImageConfig;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::image::ImageKind;

/// Domain opers for image lifecycle management: generates unique identifiers for scheduled image deletion and integrity check tasks.
pub struct ImageComplex;

impl ImageComplex {
    // Number of bytes in one mebibyte.
    const BYTES_PER_MIB: u64 = 1024 * 1024;

    /// Validates the content length against the per-kind upper bound.
    ///
    pub fn ensure_byte_length(
        image_config: &ImageConfig,
        byte_length: u64,
        kind: ImageKind,
    ) -> BaseRest<()> {
        //
        let max_length = image_config
            .limit_for(kind)
            .saturating_mul(Self::BYTES_PER_MIB);

        if !(1..=max_length).contains(&byte_length) {
            //
            let err_message = trl("error-invalid-image-byte-length");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                byte_length = byte_length,
                max_length = max_length,
                image_kind = ?kind,
                "expected error: image byte length is invalid",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            });
        }

        accept(())
    }

    /// Generate a unique image deletion-task identifier backed by a snowflake value.
    pub fn gen_delete_id() -> String {
        next_snowflake_id()
    }

    /// Generate a unique image integrity-check identifier backed by a snowflake value.
    pub fn gen_check_id() -> String {
        next_snowflake_id()
    }
}
