//! Val DTOs for the member domain.

//! Data transfer objects for member use cases.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Return value from creating a member.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateMemberVal {
    /// Identifier of the created member.
    pub id: String,
}
