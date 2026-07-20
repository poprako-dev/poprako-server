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

use crate::value::image::{ImageExtension, ImageHash};

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
    pub id: String,

    pub chapter_id: String,
    pub index: i32,

    pub image_key: Option<String>,
    pub image_uploaded: bool,
    pub image_version: u32,
    pub image_hash: ImageHash,
    pub image_byte_length: u64,
    pub image_extension: ImageExtension,

    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// The data needed to insert one page row.
#[cfg_attr(test, derive(Clone))]
pub struct PageEntry {
    pub id: String,

    pub chapter_id: String,
    pub index: i32,

    pub image_key: Option<String>,
    pub image_version: u32,
    pub image_hash: ImageHash,
    pub image_byte_length: u64,
    pub image_extension: ImageExtension,
}

/// Image reservation result for a page.
#[cfg_attr(test, derive(Clone))]
pub struct PageImageReservation {
    pub object_key: String,
    pub prev_object_key: Option<String>,
    pub image_version: u32,
}

/// Persisted manifest state for one retained or newly created page.
pub struct PageManifestUpdate {
    pub id: String,
    pub index: i32,

    pub image_key: Option<String>,
    pub image_uploaded: bool,
    pub image_version: u32,
    pub image_hash: ImageHash,
    pub image_byte_length: u64,
    pub image_extension: ImageExtension,
}
