//! Val DTOs for the workset domain.

//! Data transfer objects for workset use cases — input parameters and
//! presentation-ready values for the workset aggregate.
//!
//! Timestamps are converted to Unix milliseconds for JSON serialisation.

use serde::Serialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Return value from a successful workset creation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateWorksetVal {
    /// Identifier of the newly created workset.
    pub id: String,
}
