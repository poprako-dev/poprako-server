use time::OffsetDateTime;

use crate::result::{ExpectedVariant, RootError};

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

pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}
