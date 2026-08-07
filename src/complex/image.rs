//! Complex domain logic for image lifecycle tracking — deletion and integrity-check ID generation for asynchronous image processing.

use poprako_util::i18n::trl;

use crate::part::prom::payload::image::ResourceKind;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;

/// Domain opers for image lifecycle management: generates unique identifiers for scheduled image deletion and integrity check tasks.
pub struct ImageComplex;

impl ImageComplex {
    /// Validates the content length against the per-kind upper bound.
    ///
    /// | Kind         | Max         |
    /// |--------------|-------------|
    /// | `UserAvatar` | 512 KiB     |
    /// | `TeamAvatar` | 512 KiB     |
    /// | `ComicCover` | 2 MiB       |
    /// | `PageImage`  | 25 MiB       |
    pub fn ensure_byte_length(
        byte_length: u64,
        kind: ResourceKind,
    ) -> BaseRest<()> {
        //
        let max_length = match kind {
            //
            ResourceKind::UserAvatar | ResourceKind::TeamAvatar => 512 * 1024,

            ResourceKind::ComicCover => 2 * 1024 * 1024,

            ResourceKind::PageImage => 25 * 1024 * 1024,
        };

        if !(1..=max_length).contains(&byte_length) {
            //
            let err_message = trl("error-invalid-image-byte-length");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                byte_length = byte_length,
                max_length = max_length,
                resource_kind = ?kind,
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
