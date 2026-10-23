//! Borrowed chapter write payloads.
//! Owners remain alive through persistence; existing data is not copied.

/// The data needed to insert a new chapter row.
///
/// The `is_pinned` initial value is determined by the caller
/// (usually `true` for the first chapter in a comic, or the new
/// highest-index chapter).
///
/// Existing foreign identifiers borrow their caller-owned storage.
#[cfg_attr(test, derive(Clone))]
pub struct ChapterEntryShape<'a> {
    /// Unique identifier to insert for the new chapter.
    pub id: String,

    /// Foreign key identifying the parent comic.
    pub comic_id: &'a str,

    /// Whether the new chapter should become the active chapter immediately.
    pub is_pinned: bool,
    /// Ordinal position assigned to the new chapter within the comic.
    pub index: usize,
    /// Chapter subtitle or number for display.
    pub subtitle: String,

    /// Foreign key identifying the user creating the chapter.
    pub creator_id: &'a str,
}

impl ChapterEntryShape<'_> {
    /// Borrows normalized subtitle without copying its storage.
    pub fn trimmed_subtitle(&self) -> &str {
        self.subtitle.trim()
    }
}
