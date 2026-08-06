//! Domain models for terms stored inside terminology bases.

/// Filtering and pagination parameters for terms inside one terminology base.
pub struct TermListSpec {
    /// The terminology base whose terms are being queried.
    pub termbase_id: String,

    /// Optional substring filter against the source term text.
    pub fuzzy_source: Option<String>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return in this page.
    pub limit: u32,
}
