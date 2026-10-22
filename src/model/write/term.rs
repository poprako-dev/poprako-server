//! Domain models for terms stored inside terminology bases.

/// The data needed to create a terminology entry.
#[cfg_attr(test, derive(Clone))]
pub struct TermEntry<'a> {
    /// The unique identifier for the new terminology entry.
    pub id: String,

    /// Foreign key of the terminology base to insert this term into.
    pub termbase_id: String,

    /// The source-language term or phrase to add.
    pub source: &'a str,
    /// One or more target-language translations for the new term.
    pub targets: Vec<&'a str>,
    /// Optional annotation or usage note for the new term.
    pub comment: Option<&'a str>,

    /// Foreign key of the user creating this term entry.
    pub creator_id: String,
}

/// Portable terminology-entry content before persistence identity is assigned.
#[cfg_attr(test, derive(Clone))]
pub struct TermImport<'a> {
    /// Source-language term or phrase.
    pub source: &'a str,
    /// Target-language translations.
    pub targets: Vec<&'a str>,
    /// Optional annotation or usage note.
    pub comment: Option<&'a str>,
}

/// Mutable terminology-entry fields.
#[cfg_attr(test, derive(Clone))]
pub struct TermRepl<'a> {
    /// The unique identifier of the term to update.
    pub id: String,

    /// Updated source-language term or phrase.
    pub source: &'a str,
    /// Updated target-language translations for the term.
    pub targets: Vec<&'a str>,
    /// Updated annotation or usage note for the term.
    pub comment: Option<&'a str>,
}

/// Delivery changes produced by one terminology-base import.
pub struct TermUpsertPlan<'a> {
    /// New terminology entries to insert.
    pub entries: Vec<TermEntry<'a>>,

    /// Existing terminology entries to update.
    pub updates: Vec<TermRepl<'a>>,
}
