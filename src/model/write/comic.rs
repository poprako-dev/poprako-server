//! Domain models for comics inside worksets — core metadata, cover-storage
//! tracking, and denormalised chapter counters.
//!
//! Convert to [`ComicInfoView`] for presentation outside the domain layer.
//!
//! [`ComicInfoView`]: crate::data::view::comic::ComicInfoView

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
    pub index: usize,

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
