//! Val DTOs for the user domain.

//! Data transfer objects for user profile use cases.

use serde::Serialize;

use crate::data::view::image::ImageUploadSlotView;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// User avatar upload reservation response.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveUserAvatarVal {
    /// Upload capability, absent when this avatar is already uploaded.
    pub slot: Option<ImageUploadSlotView>,
}
