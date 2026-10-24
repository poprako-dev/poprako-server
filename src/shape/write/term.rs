//! Batch plans borrowing import models and persisted terms through the write.
//! Target lists copy references; append replacements own their merged comments.

/// The data needed to create a terminology entry.
#[cfg_attr(test, derive(Clone))]
pub struct TermEntryShape<'a> {
    /// The unique identifier for the new terminology entry.
    pub id: String,

    /// Foreign key of the terminology base to insert this term into.
    pub termbase_id: &'a str,

    /// The source-language term or phrase to add.
    pub source: &'a str,
    /// One or more target-language translations for the new term.
    pub targets: Vec<&'a str>,
    /// Optional annotation or usage note for the new term.
    pub comment: Option<&'a str>,

    /// Foreign key of the user creating this term entry.
    pub creator_id: &'a str,
}

/// Mutable terminology-entry fields.
#[cfg_attr(test, derive(Clone))]
pub struct TermReplShape<'a> {
    /// The unique identifier of the term to update.
    pub id: &'a str,

    /// Updated source-language term or phrase.
    pub source: &'a str,
    /// Updated target-language translations for the term.
    pub targets: Vec<&'a str>,
    /// Updated annotation or usage note for the term.
    pub comment: Option<&'a str>,
}

impl<'a> From<&'a TermAppendReplShape<'a>> for TermReplShape<'a> {
    // Borrow merged content for the existing typed persistence operation.
    fn from(update: &'a TermAppendReplShape<'a>) -> Self {
        Self {
            id: update.id,
            source: update.source,
            targets: update.targets.clone(),
            comment: update.comment.as_deref(),
        }
    }
}

/// Delivery changes produced by one terminology-base import.
pub struct TermUpsertPlanShape<'a> {
    /// New terminology entries to insert.
    pub entries: Vec<TermEntryShape<'a>>,

    /// Existing terminology entries to update.
    pub updates: Vec<TermReplShape<'a>>,
}

/// A merged terminology replacement owning its combined comment text.
pub struct TermAppendReplShape<'a> {
    /// Identifier of the existing terminology entry.
    pub id: &'a str,

    /// Original source spelling retained by the append.
    pub source: &'a str,
    /// Existing translations followed by new distinct translations.
    pub targets: Vec<&'a str>,
    /// Combined complete comment blocks.
    pub comment: Option<String>,
}

/// Changes produced by appending a validated terminology batch.
pub struct TermAppendPlanShape<'a> {
    /// New terminology entries to insert.
    pub entries: Vec<TermEntryShape<'a>>,
    /// Existing entries whose content actually changes.
    pub updates: Vec<TermAppendReplShape<'a>>,
}
