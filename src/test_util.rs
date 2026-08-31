//! Test helpers for assertions and time fixtures.

pub mod fixture;

use time::OffsetDateTime;

use poprako_util::i18n::trl;

use crate::ImageConfig;
use crate::result::{BaseError, ExpectedVariant};

/// Image limits matching the default runtime configuration.
pub const IMAGE_CONFIG: ImageConfig = ImageConfig {
    user_avatar_limit: 1,
    team_avatar_limit: 1,
    comic_cover_limit: 2,
    page_image_limit: 25,
};

/// Asserts that `err` is a [`RootError::Expected`] whose variant matches `expected`.
/// Panics with a descriptive message on mismatch.
pub fn assert_expected_variant(err_: BaseError, expected: ExpectedVariant) {
    //
    let BaseError::Expected { variant, .. } = err_ else {
        panic!("expected RootError::Expected");
    };

    match (variant, expected) {
        //
        (ExpectedVariant::Args, ExpectedVariant::Args)
        | (ExpectedVariant::Auth, ExpectedVariant::Auth)
        | (ExpectedVariant::Perm, ExpectedVariant::Perm) => {}

        _ => panic!("unexpected ExpectedVariant"),
    }
}

/// Asserts that `err` is an expected error with the exact variant and i18n key.
pub fn assert_expected_message(
    err_: BaseError,
    expected: ExpectedVariant,
    trl_key: &str,
) {
    //
    let BaseError::Expected {
        variant,
        message: actual,
    } = err_
    else {
        panic!("expected RootError::Expected");
    };

    assert_expected_variant(
        BaseError::Expected {
            variant,
            message: actual.clone(),
        },
        expected,
    );

    assert_eq!(actual, trl(trl_key));
}

/// Returns the current time in UTC. Convenience wrapper for tests.
pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}
