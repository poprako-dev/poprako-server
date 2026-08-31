//! Val DTOs for the team domain.

//! Data transfer objects for team profile use cases.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::image::ImageUploadSlotView;

/// Team avatar upload allocation response.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocTeamAvatarVal {
    /// Upload capability, absent when this avatar is already uploaded.
    pub slot: Option<ImageUploadSlotView>,
}
