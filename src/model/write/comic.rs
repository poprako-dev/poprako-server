//! Domain models for comics inside worksets — core metadata, cover-storage
//! tracking, and denormalised chapter counters.
//!
//! Convert to [`ComicInfoVal`] for presentation outside the domain layer.
//!
//! [`ComicInfoVal`]: crate::data::val::comic::ComicInfoVal

/// The data needed to insert a new comic row.
///
/// Supplied at comic-creation time. The `id` is typically generated via
/// [`ComicComplex::gen_id`]; the `index` is allocated by the repo layer.
///
/// [`ComicComplex::gen_id`]: crate::complex::comic::ComicComplex::gen_id
#[cfg_attr(test, derive(Clone))]
pub struct ComicEntry {
    //
    /// Unique identifier for the new comic.
    pub id: String,

    /// The workset this comic will be created under.
    pub workset_id: String,
    /// Sorting position assigned by the insertion logic.
    pub index: i32,

    /// Display title of the comic series or volume.
    pub title: String,
    /// Name of the author or artist responsible for this comic.
    pub author: String,
    /// Optional synopsis or editorial notes to accompany the comic.
    pub description: Option<String>,

    /// The user who creates this comic record.
    pub creator_id: String,
}

/// Mutable profile (non-cover, non-counter) fields for a comic.
#[cfg_attr(test, derive(Clone))]
pub struct ComicRepl {
    //
    /// Identifies which comic record to update.
    pub id: String,

    /// Updated display title of the comic series or volume.
    pub title: String,
    /// Updated name of the author or artist.
    pub author: String,
    /// Updated synopsis or editorial notes.
    pub description: Option<String>,
}

/// The result of reserving a new comic cover upload slot.
#[cfg_attr(test, derive(Clone))]
pub struct ComicCoverReservation {
    //
    /// Newly generated object-storage key for the cover upload.
    pub object_key: String,

    /// Previous cover key to clean up after a successful replacement.
    pub prev_object_key: Option<String>,

    /// Version that must match when the upload is confirmed.
    pub cover_version: u32,

    /// Whether a new upload is required.
    pub is_upload_required: bool,
}
