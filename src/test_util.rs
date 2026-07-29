//! Test helpers for assertions and time fixtures.

use time::OffsetDateTime;

use poprako_util::i18n::trl;

use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::image::{ImagePayload, ResourceKind};
use crate::part_impl::prom::mock_impl::MockPromRecord;
use crate::result::{BaseError, ExpectedVariant};

pub mod fixture;

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

/// Counts exact image upload-check prom records.
pub fn count_image_check_records(
    records: &[MockPromRecord],
    resource_kind: ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
) -> usize {
    records
        .iter()
        .filter(|record| {
            matches!(
                record.payload(),
                TaskPayload::Image(ImagePayload::CheckUpload {
                    resource_kind: actual_resource_kind,
                    resource_id: actual_resource_id,
                    object_key: actual_object_key,
                    version: actual_version,
                    ..
                }) if actual_resource_kind == resource_kind
                    && actual_resource_id == resource_id
                    && actual_object_key == object_key
                    && actual_version == image_version
            )
        })
        .count()
}

/// Asserts that exactly one matching image upload-check prom record exists.
pub fn assert_one_image_check_record(
    records: &[MockPromRecord],
    resource_kind: ResourceKind,
    resource_id: &str,
    object_key: &str,
    image_version: u32,
) {
    assert_eq!(
        count_image_check_records(
            records,
            resource_kind,
            resource_id,
            object_key,
            image_version
        ),
        1
    );
}

/// Returns the current time in UTC. Convenience wrapper for tests.
pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}
