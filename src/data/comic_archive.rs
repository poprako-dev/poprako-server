//! Response values for immutable comic archive operations.

use serde::Serialize;

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

/// Value returned after an active comic has been archived atomically.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct Val {
    pub archived_comic_id: String,
}
