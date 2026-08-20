//! Val DTOs for the term domain.

//! Request and response DTOs for terminology-entry use cases.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Return value from creating a terminology entry.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateTermVal {
    /// Identifier of the newly created terminology entry.
    pub id: String,
}
