//! Val DTOs for the comment domain.

//! Data transfer objects for comment use cases.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Return value from creating a comment.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateCommentVal {
    /// Identifier of the newly created comment.
    pub id: String,
}
