//! Domain models for pages inside chapters — per-page unit tracking and
//! image-storage metadata.
//!
//! Each page belongs to exactly one chapter and carries denormalised progress
//! counters that aggregate to the parent chapter's overview totals.
//!
//! Convert to [`PageInfoVal`] for presentation.
//!
//! [`PageInfoVal`]: crate::data::page::PageInfoVal

use time::OffsetDateTime;

use crate::value::image::{ImageExt, ImageHash};

/// A pagerecord as stored in the database.
///
/// Progress is tracked via three denormalised counters (`total_unit_count`,
/// `translated_unit_count`, `proofread_unit_count`) that roll up to the
/// parent [`ChapterInfo`] totals.
///
/// The `image_key` and `image_uploaded` fields track the page image
/// lifecycle: a key is reserved when the page is created, the client
/// uploads the image, then the upload is confirmed.
///
/// [`ChapterInfo`]: crate::model::chapter::ChapterInfo
#[cfg_attr(test, derive(Clone))]
pub struct PageInfo {
    //
    /// The unique identifier for this page record.
    pub id: String,

    /// Foreign key to the parent chapter this page belongs to.
    pub chapter_id: String,
    /// Zero-based ordinal position of this page within the chapter.
    pub index: i32,

    /// Object-storage key reserved for the page image, `None` before reservation.
    pub image_key: Option<String>,
    /// Whether the client has confirmed the image upload for this page.
    pub image_uploaded: bool,
    /// Monotonically increasing version counter, bumped on each image reservation.
    pub image_version: u32,
    /// Content-addressable hash of the uploaded image file.
    pub image_hash: ImageHash,
    /// File format.
    pub image_ext: ImageExt,

    /// Number of translation units (text blocks) on this page.
    pub total_unit_count: i32,
    /// Number of units on this page that have been translated.
    pub translated_unit_count: i32,
    /// Number of units on this page that have been proofread.
    pub proofread_unit_count: i32,

    /// Timestamp when this page record was first created.
    pub created_at: OffsetDateTime,
    /// Timestamp when this page record was last modified.
    pub updated_at: OffsetDateTime,
}

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

/// Persisted manifest state for one retained or newly created page.
pub struct PageManifestUpdate {
    //
    /// The unique identifier of the page whose manifest is being updated.
    pub id: String,
    /// Updated ordinal position of the page within the chapter.
    pub index: i32,

    /// Updated object-storage key for the page image.
    pub image_key: Option<String>,
    /// Whether the image upload has been confirmed for this page.
    pub image_uploaded: bool,
    /// Updated version counter for the image lifecycle.
    pub image_version: u32,
    /// Updated content hash of the page image file.
    pub image_hash: ImageHash,
    /// Updated file format extension of the page image.
    pub image_ext: ImageExt,
}
