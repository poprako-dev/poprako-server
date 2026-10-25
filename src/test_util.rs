//! Test helpers for assertions and time fixtures.

mod logging_tests;

pub mod fixture;
/// HTTP response and tracing capture for boundary regression tests.
pub mod http_logging;

use time::OffsetDateTime;

use poprako_util::i18n::trl;

use crate::ImageConfig;
use crate::result::{BaseError, ExpectedVariant};

/// Image limits matching the default runtime configuration.
pub const IMAGE_CONFIG: ImageConfig = ImageConfig {
    user_avatar_limit_mib: 1,
    team_avatar_limit_mib: 1,
    comic_cover_limit_mib: 2,
    page_source_image_limit_mib: 25,
    page_artwork_image_limit_mib: 25,
};

/// Asserts that `error` is a [`BaseError::Expected`] whose variant matches `expected`.
/// Panics with a descriptive message on mismatch.
pub fn assert_expected_variant(error_: &BaseError, expected: ExpectedVariant) {
    //
    assert!(matches!(
        (error_, expected),
        (
            BaseError::Expected {
                variant: ExpectedVariant::Args,
                ..
            },
            ExpectedVariant::Args,
        ) | (
            BaseError::Expected {
                variant: ExpectedVariant::Auth,
                ..
            },
            ExpectedVariant::Auth,
        ) | (
            BaseError::Expected {
                variant: ExpectedVariant::Perm,
                ..
            },
            ExpectedVariant::Perm,
        )
    ));
}

/// Asserts that `error` is an expected error with the exact variant and i18n key.
pub fn assert_expected_message(
    error_: BaseError,
    expected: ExpectedVariant,
    trl_key: &str,
) {
    //
    assert_expected_variant(&error_, expected);

    let actual = match error_ {
        //
        BaseError::Expected { message, .. } => Some(message),

        _ => None,
    };

    assert_eq!(actual, Some(trl(trl_key)));
}

/// Returns the current time in UTC. Convenience wrapper for tests.
pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}
