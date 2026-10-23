//! Domain models for comics inside worksets — core metadata, cover-storage
//! tracking, and denormalised chapter counters.
//!
//! Convert to [`ComicInfoView`] for presentation outside the domain layer.
//!
//! [`ComicInfoView`]: crate::data::view::comic::ComicInfoView

/// Mutable profile (non-cover, non-counter) fields for a comic.
#[cfg_attr(test, derive(Clone))]
pub struct ComicRepl {
    /// Identifies which comic record to update.
    pub id: String,

    /// Updated display title of the comic series or volume.
    pub title: String,
    /// Updated name of the author or artist.
    pub author: String,
    /// Updated synopsis or editorial notes.
    pub description: Option<String>,
}

impl ComicRepl {
    /// Borrows normalized title without copying its storage.
    pub fn trimmed_title(&self) -> &str {
        self.title.trim()
    }

    /// Borrows normalized author without copying its storage.
    pub fn trimmed_author(&self) -> &str {
        self.author.trim()
    }

    /// Borrows normalized description without copying its storage.
    pub fn trimmed_description(&self) -> Option<&str> {
        self.description.as_deref().map(str::trim)
    }
}
