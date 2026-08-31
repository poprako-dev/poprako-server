//! Domain models for pages inside chapters — per-page unit tracking and
//! image-storage metadata.
//!
//! Each page belongs to exactly one chapter and carries denormalised progress
//! counters that aggregate to the parent chapter's overview totals.
//!
//! Convert to [`PageInfoView`] for presentation.
//!
//! [`PageInfoView`]: crate::data::view::page::PageInfoView

use time::OffsetDateTime;

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
/// [`ChapterInfo`]: crate::model::read::proj::chapter::ChapterInfo
#[cfg_attr(test, derive(Clone))]
pub struct PageInfo {
    //
    /// The unique identifier for this page record.
    pub id: String,

    /// Foreign key to the parent chapter this page belongs to.
    pub chapter_id: String,
    /// Zero-based ordinal position of this page within the chapter.
    pub index: usize,

    /// Number of translation units (text blocks) on this page.
    pub total_unit_count: usize,
    /// Number of units on this page that have been translated.
    pub translated_unit_count: usize,
    /// Number of units on this page that have been proofread.
    pub proofread_unit_count: usize,

    /// Timestamp when this page record was first created.
    pub created_at: OffsetDateTime,
    /// Timestamp when this page record was last modified.
    pub updated_at: OffsetDateTime,
}
