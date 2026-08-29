//! Complex domain logic for image lifecycle tracking — deletion and integrity-check ID generation for asynchronous image processing.

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use poprako_util::i18n::trl_kv;

use crate::config::image::ImageConfig;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
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
        let max_mib = image_config.limit_for(kind);

        let max_length = max_mib.saturating_mul(Self::BYTES_PER_MIB);

        if !(1..=max_length).contains(&byte_length) {
            //
            return Err(Self::invalid_byte_length_rejection(
                image_config,
                byte_length,
                kind,
            ));
        }

        accept(())
    }

    /// Builds the client-correctable rejection for a missing or invalid image length.
    pub fn invalid_byte_length_rejection(
        image_config: &ImageConfig,
        byte_length: u64,
        kind: ImageKind,
    ) -> BaseError {
        //
        let max_mib = image_config.limit_for(kind);

        let args = HashMap::from([
            ("min_bytes".into(), 1_u64.into()),
            ("max_mib".into(), max_mib.into()),
        ]);

        let err_message = trl_kv("error-invalid-image-byte-length", &args);

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            byte_length,
            max_length = max_mib.saturating_mul(Self::BYTES_PER_MIB),
            image_kind = ?kind,
            "expected error: image byte length is invalid",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        }
    }
}
