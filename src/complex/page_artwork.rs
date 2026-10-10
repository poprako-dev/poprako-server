//! Pure composite page manifest validation.

/// Composite page write permission rules.
pub mod perm;

use std::collections::HashSet;

use poprako_util::i18n::trl;

use crate::data::instr::page_artwork::PageArtworkImageInstr;
use crate::model::read::proj::page_artwork::PageArtworkInfo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Produces a localized composite page rejection.
pub fn error(variant: ExpectedVariant, key: &str) -> BaseError {
    //
    let msg = trl(key);

    tracing::warn!(err_variant = ?variant, err_msg = %msg, "composite page rejected");

    BaseError::Expected { variant, msg }
}

/// Explicit IDs must be distinct identities from the locked Chapter manifest.
pub fn ensure_manifest(
    pages: &[PageArtworkImageInstr],
    existing: &[PageArtworkInfo],
) -> BaseRest<()> {
    //
    let mut ids = HashSet::new();

    for page in pages {
        //
        let Some(id) = &page.page_artwork_id else {
            continue;
        };

        if !ids.insert(id) || !existing.iter().any(|info| info.id == *id) {
            //
            return Err(error(
                ExpectedVariant::Args,
                "error-page-artwork-target",
            ));
        }
    }

    accept(())
}
