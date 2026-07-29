//! Val DTOs for the chapter domain.

//! Data transfer objects for chapter use cases.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Return value from a successful chapter creation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateChapterVal {
    /// Unique identifier of the newly created chapter.
    pub id: String,
}
