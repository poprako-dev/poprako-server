//! Domain models for comics inside worksets — core metadata, cover-storage
//! tracking, and denormalised chapter counters.
//!
//! A comic is the primary grouping entity inside a workset: it owns chapters,
//! pages, and a multi-step cover image upload flow. The `chapter_count` and
//! `chapter_next_index` fields are denormalised counters refreshed by the
//! chapter creation/deletion pipeline.
//!
//! Convert to [`ComicInfoVal`] for presentation outside the domain layer.
//!
//! [`ComicInfoVal`]: crate::data::comic::ComicInfoVal

use time::OffsetDateTime;

/// A comic（漫画）record as stored in the database.
///
/// Each comic belongs to exactly one workset. The `is_completed` flag
/// toggles whether the comic is treated as finished in list views.
/// Cover uploads follow a multi-step flow: a key is reserved via
/// [`ComicStep::reserve_cover`], the client uploads to that key, then
/// the upload is confirmed via [`ComicStep::mark_cover_uploaded`].
///
/// [`ComicStep::reserve_cover`]: crate::part::repo::step::comic::ComicStep::reserve_cover
/// [`ComicStep::mark_cover_uploaded`]: crate::part::repo::step::comic::ComicStep::mark_cover_uploaded
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

/// The data needed to insert a new comic row.
///
/// Supplied at comic-creation time. The `id` is typically generated via
/// [`ComicComplex::gen_id`]; the `index` is allocated by incrementing the
/// parent workset's `comic_next_index` counter in the same transaction.
///
/// [`ComicComplex::gen_id`]: crate::complex::comic::ComicComplex::gen_id
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

/// Mutable profile (non-cover, non-counter) fields for a comic.
#[cfg_attr(test, derive(Clone))]
pub struct ComicInfoUpdate {
    pub id: String,

    pub title: String,
    pub author: String,
    pub description: Option<String>,
}

/// The result of reserving a new comic cover upload slot.
///
/// Mirrors [`TeamAvatarReservation`] for the comic domain. Contains the
/// generated object-storage key for the client to PUT to, the previous key
/// (if any) to clean up after the new upload succeeds, and the version
/// number that must match when marking the upload complete.
///
/// [`TeamAvatarReservation`]: crate::model::team::TeamAvatarReservation
#[cfg_attr(test, derive(Clone))]
pub struct ComicCoverReservation {
    pub object_key: String,
    pub prev_object_key: Option<String>,
    pub cover_version: i64,
}
