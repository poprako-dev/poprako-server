//! Test helpers for assertions and time fixtures.

use time::OffsetDateTime;

use crate::result::{ExpectedVariant, RootError};

/// Asserts that `err` is a [`RootError::Expected`] whose variant matches `expected`.
/// Panics with a descriptive message on mismatch.
pub fn assert_expected_variant(err: RootError, expected: ExpectedVariant) {
    let RootError::Expected { variant, .. } = err else {
        panic!("expected RootError::Expected");
    };

    match (variant, expected) {
        (ExpectedVariant::Args, ExpectedVariant::Args)
        | (ExpectedVariant::Auth, ExpectedVariant::Auth)
        | (ExpectedVariant::Perm, ExpectedVariant::Perm) => {}
        _ => panic!("unexpected ExpectedVariant"),
    }
}

/// Returns the current time in UTC. Convenience wrapper for tests.
pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}
