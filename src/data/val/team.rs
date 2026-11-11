//! Val DTOs for the team domain.
//! Data transfer objects for team profile use cases.
use crate::data::view::image::ImageUploadSlotView;

/// Team avatar upload allocation response.
pub struct AllocTeamAvatarVal {
    /// Upload capability, absent when this avatar is already uploaded.
    pub slot: Option<ImageUploadSlotView>,
}
