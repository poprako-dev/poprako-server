//! Domain models for pages inside chapters.

use time::OffsetDateTime;

/// A page record as stored in the database.
#[cfg_attr(test, derive(Clone))]
pub struct PageInfo {
    pub id: String,
    pub chapter_id: String,
    pub image_key: Option<String>,
    pub image_uploaded: bool,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
