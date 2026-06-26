//! Domain models for comics inside worksets.

use time::OffsetDateTime;

/// A comic record as stored in the database.
#[cfg_attr(test, derive(Clone))]
pub struct ComicInfo {
    pub id: String,

    pub workset_id: String,
    pub index: i32,

    pub title: String,
    pub author: String,
    pub description: Option<String>,
    pub is_completed: bool,

    pub cover_key: Option<String>,
    pub cover_uploaded: bool,
    pub cover_version: i64,

    pub chapter_count: i32,
    pub chapter_next_index: i32,

    pub creator_id: String,
    pub last_active_at: OffsetDateTime,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// The data needed to create a new comic.
#[cfg_attr(test, derive(Clone))]
pub struct ComicForm {
    pub id: String,

    pub workset_id: String,
    pub index: i32,

    pub title: String,
    pub author: String,
    pub description: Option<String>,

    pub creator_id: String,
}

/// Mutable profile fields for a comic.
#[cfg_attr(test, derive(Clone))]
pub struct ComicInfoUpdate {
    pub id: String,

    pub title: String,
    pub author: String,
    pub description: Option<String>,
}

/// The result of reserving a new comic cover upload slot.
#[cfg_attr(test, derive(Clone))]
pub struct ComicCoverReservation {
    pub object_key: String,
    pub previous_object_key: Option<String>,
    pub cover_version: i64,
}
