//! Capabilities for composite artwork pages.

use poprako_orchestra::drive;

use crate::part::repo::oper::page_artwork::{
    GetPageArtworkInfo, ListPageArtworkInfos, ReplacePageArtworkManifest,
    UpdatePageArtworkInfo,
};
use crate::result::BaseError;

/// Composite page reads and coordinated manifest replacement.
#[drive(
    context = C,
    error = BaseError,
    run(for<'a> GetPageArtworkInfo<'a>, for<'a> ListPageArtworkInfos<'a>),
    step(for<'a> ListPageArtworkInfos<'a>, for<'a> ReplacePageArtworkManifest<'a>, for<'a> UpdatePageArtworkInfo<'a>),
)]
pub trait PageArtworkRepo<C> {}
