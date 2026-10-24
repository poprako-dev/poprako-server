//! Borrowed comic write payloads.
//! Owners remain alive through persistence; existing data is not copied.

/// The data needed to insert a new comic row.
///
/// Existing foreign identifiers borrow their caller-owned storage.
#[cfg_attr(test, derive(Clone))]
pub struct ComicEntryShape<'a> {
    /// Unique identifier for the new comic.
    pub id: String,

    /// The workset this comic will be created under.
    pub workset_id: &'a str,
    /// Sorting position assigned by the insertion logic.
    pub index: usize,

    /// Display title of the comic series or volume.
    pub title: String,
    /// Name of the author or artist responsible for this comic.
    pub author: String,
    /// Optional synopsis or editorial notes to accompany the comic.
    pub description: Option<String>,

    /// The user who creates this comic record.
    pub creator_id: &'a str,
}

impl ComicEntryShape<'_> {
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
