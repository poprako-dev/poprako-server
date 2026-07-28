//! Domain models for pages inside chapters — per-page unit tracking and
//! image-storage metadata.
//!
//! Each page belongs to exactly one chapter and carries denormalised progress
//! counters that aggregate to the parent chapter's overview totals.
//!
//! Convert to [`PageInfoVal`] for presentation.
//!
//! [`PageInfoVal`]: crate::data::val::page::PageInfoVal

use crate::value::image::{ImageExt, ImageHash};

/// The data needed to insert one page row.
#[cfg_attr(test, derive(Clone))]
pub struct PageEntry {
    //
    /// The unique identifier for the new page record.
    pub id: String,

    /// Foreign key of the parent chapter to insert this page into.
    pub chapter_id: String,
    /// Zero-based ordinal position for this page within the chapter.
    pub index: i32,

    /// Object-storage key reserved for the page image.
    pub image_key: Option<String>,
    /// Starting version counter for the image lifecycle.
    pub image_version: u32,
    /// Content-addressable hash of the initial image file.
    pub image_hash: ImageHash,
    /// File format extension of the initial page image.
    pub image_ext: ImageExt,
}

/// One page-image identity supplied to manifest planning.
pub struct PageImageSpec {
    //
    /// Existing page identifier, if the manifest retains a known page.
    pub page_id: Option<String>,
    /// Content-addressable hash of the page image file.
    pub image_hash: ImageHash,
    /// File size when this manifest entry requests an upload slot.
    pub new_byte_len: Option<u64>,
    /// File format of the page image.
    pub ext: ImageExt,
}

/// Image reservation result for a page.
#[cfg_attr(test, derive(Clone))]
pub struct PageImageReservation {
    //
    /// Newly generated object-storage key for the image upload slot.
    pub object_key: String,
    /// Previous image key that should be cleaned up from storage, if any.
    pub prev_object_key: Option<String>,
    /// The new version number that must match on upload confirmation.
    pub image_version: u32,
}

/// A page image upload state replacement.
pub struct PageImageRepl {
    //
    /// The page identifier.
    pub id: String,

    /// The image version being confirmed.
    pub image_version: u32,
    /// The object-storage key, when the image has an object identity.
    pub image_key: Option<String>,
    /// Whether the image upload has completed.
    pub is_image_uploaded: bool,
}

/// Persisted manifest state for one retained or newly created page.
/// TODO: why is this necessary?
pub struct PageManifestRepl {
    //
    /// The unique identifier of the page whose manifest is being updated.
    pub id: String,
    /// Updated ordinal position of the page within the chapter.
    pub index: i32,

    /// Updated object-storage key for the page image.
    pub image_key: Option<String>,
    /// Whether the image upload has been confirmed for this page.
    pub is_image_uploaded: bool,
    /// Updated version counter for the image lifecycle.
    pub image_version: u32,
    /// Updated content hash of the page image file.
    pub image_hash: ImageHash,
    /// Updated file format extension of the page image.
    pub image_ext: ImageExt,
}
