//! Val DTOs for the announcement domain.

//! Data transfer objects for announcement use cases.

use serde::Serialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Return value from creating an announcement.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateAnnouncementVal {
    /// Identifier of the created announcement.
    pub id: String,
}
