//! Val DTOs for the termbase domain.

//! Request and response DTOs for terminology-base use cases.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Return value from creating a terminology base.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[cfg_attr(test, derive(Debug))]
pub struct CreateTermbaseVal {
    /// Identifier of the newly created terminology base.
    pub id: String,
}
