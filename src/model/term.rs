//! Domain models for terms stored inside terminology bases.

use time::OffsetDateTime;

/// A persisted terminology entry.
#[cfg_attr(test, derive(Clone))]
pub struct TermInfo {
    //
    /// The unique identifier for this terminology entry.
    pub id: String,

    /// Foreign key to the terminology base this term belongs to.
    pub termbase_id: String,

    /// The source-language term or phrase.
    pub source: String,
    /// One or more target-language translations for the source term.
    pub targets: Vec<String>,
    /// Optional annotation or usage note for this terminology entry.
    pub comment: Option<String>,

    /// Foreign key to the user who created this term entry.
    pub creator_id: String,

    /// Timestamp when this term was first created.
    pub created_at: OffsetDateTime,
    /// Timestamp when this term was last modified.
    pub updated_at: OffsetDateTime,
}

/// The data needed to create a terminology entry.
#[cfg_attr(test, derive(Clone))]
pub struct TermEntry {
    //
    /// The unique identifier for the new terminology entry.
    pub id: String,

    /// Foreign key of the terminology base to insert this term into.
    pub termbase_id: String,

    /// The source-language term or phrase to add.
    pub source: String,
    /// One or more target-language translations for the new term.
    pub targets: Vec<String>,
    /// Optional annotation or usage note for the new term.
    pub comment: Option<String>,

    /// Foreign key of the user creating this term entry.
    pub creator_id: String,
}

/// Mutable terminology-entry fields.
#[cfg_attr(test, derive(Clone))]
pub struct TermInfoUpdate {
    //
    /// The unique identifier of the term to update.
    pub id: String,

    /// Updated source-language term or phrase.
    pub source: String,
    /// Updated target-language translations for the term.
    pub targets: Vec<String>,
    /// Updated annotation or usage note for the term.
    pub comment: Option<String>,
}

/// Filtering and pagination parameters for terms inside one terminology base.
pub struct TermInfoListSpec {
    //
    /// The terminology base whose terms are being queried.
    pub termbase_id: String,

    /// Optional substring filter against the source term text.
    pub fuzzy_source: Option<String>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return in this page.
    pub limit: u32,
}
