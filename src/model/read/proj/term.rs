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
