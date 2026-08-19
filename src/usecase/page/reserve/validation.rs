//! Chapter page-count validation.

use poprako_util::i18n::trl;

use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

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
