//! Pure composite page manifest validation.

/// Composite page write permission rules.
pub mod perm;

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::model::read::proj::page_artwork::PageArtworkInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::value::business_id::BusinessIdKind;

/// Produces a localized composite page rejection.
pub fn error(variant: ExpectedVariant, key: &str) -> BaseError {
    //
    expected_error! {
        variant,
        message: trl(key);
        "composite page rejected",
    }
}

/// Explicit IDs must be distinct identities from the locked Chapter manifest.
pub fn ensure_manifest<'a, I>(
    page_artwork_ids: I,
    existing: &[PageArtworkInfo],
) -> BaseRest<()>
where
    I: IntoIterator<Item = &'a str>,
{
    //
    let mut ids = HashSet::new();

    for id in page_artwork_ids {
        //
        if !BusinessIdKind::PageArtwork.is_valid_id(id)
            || !ids.insert(id)
            || !existing.iter().any(|info| info.id == id)
        {
            //
            return Err(error(
                ExpectedVariant::Args,
                "error-page-artwork-target",
            ));
        }
    }

    accept!(())
}
