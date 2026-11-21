//! Val DTOs for the workset domain.
//! Data transfer objects for workset use cases — input parameters and
//! presentation-ready values for the workset aggregate.
//!
//! Timestamps are converted to Unix milliseconds for JSON serialisation.
/// Return value from a successful workset creation.
pub struct CreateWorksetVal {
    /// Identifier of the newly created workset.
    pub id: String,
}
