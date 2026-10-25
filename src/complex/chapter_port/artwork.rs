//! Pure chapter artwork allocation and permission rules.

use poprako_util::i18n::trl;

use crate::config::artwork::ArtworkConfig;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::value::artwork::ArtworkExt;
use crate::value::role::RoleField;

/// Validates arbitrary file suffixes without treating them as path segments.
pub fn valid_ext(ext: &str) -> bool {
    ArtworkExt::is_valid(ext)
}

/// Constructs one classified, translated artwork error at its production leaf.
pub fn artwork_error(variant: ExpectedVariant, key: &str) -> BaseError {
    //
    expected_error! {
        variant,
        message: trl(key);
        "chapter artwork rejected",
    }
}

/// Checks the exact upload size and extension before allocating an object.
pub fn ensure_alloc(
    config: ArtworkConfig,
    byte_len: u64,
    ext: &str,
) -> BaseRest<()> {
    //
    let limit = config.chapter_artwork_limit_mib.checked_mul(1024 * 1024);

    if byte_len == 0
        || limit.is_none_or(|limit| byte_len > limit)
        || byte_len > i64::MAX as u64
        || !valid_ext(ext)
    {
        //
        return Err(artwork_error(
            ExpectedVariant::Args,
            "error-invalid-artwork-upload",
        ));
    }

    accept!(())
}

/// Requires a chapter typesetter or redrawer assignment.
pub fn ensure_user_can_upload(
    assignment_info: &AssignmentInfo,
) -> BaseRest<()> {
    //
    let can_upload = assignment_info
        .roles
        .has_any_role(&[RoleField::TYPESETTER, RoleField::REDRAWER]);

    if !can_upload {
        //
        return Err(artwork_error(
            ExpectedVariant::Perm,
            "error-artwork-upload-role-required",
        ));
    }

    accept!(())
}
