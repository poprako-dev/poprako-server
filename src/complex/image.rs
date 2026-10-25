//! Pure image upload size rules.

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use poprako_util::i18n::trl_kv;

use crate::config::image::ImageConfig;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::value::image::ImageKind;

// Number of bytes in one mebibyte.
const BYTES_PER_MIB: u64 = 1024 * 1024;

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

    expected_error! {
        ExpectedVariant::Args,
        message: trl_kv("error-invalid-image-byte-length", &args);
        byte_length,
        max_length = max_mib.saturating_mul(BYTES_PER_MIB),
        image_kind = ?kind,
        "image byte length is invalid",
    }
}

/// Validates the content length against the per-kind upper bound.
///
pub fn ensure_byte_length(
    image_config: &ImageConfig,
    byte_length: u64,
    kind: ImageKind,
) -> BaseRest<()> {
    //
    let max_mib = image_config.limit_for(kind);

    let max_length = max_mib.saturating_mul(BYTES_PER_MIB);

    if !(1..=max_length).contains(&byte_length) {
        //
        return Err(invalid_byte_length_rejection(
            image_config,
            byte_length,
            kind,
        ));
    }

    accept!(())
}
