//! Val DTOs for the team domain.

//! Data transfer objects for team profile use cases.

use serde::Serialize;

use crate::data::view::image::ImageUploadSlotView;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Team avatar upload reservation response.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveTeamAvatarVal {
    /// Upload capability, absent when this avatar is already uploaded.
    pub slot: Option<ImageUploadSlotView>,
}
