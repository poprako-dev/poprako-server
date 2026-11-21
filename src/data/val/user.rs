//! Val DTOs for the user domain.
//! Data transfer objects for user profile use cases.
use crate::data::view::image::ImageUploadSlotView;

/// User avatar upload allocation response.
pub struct AllocUserAvatarVal {
    /// Upload capability, absent when this avatar is already uploaded.
    pub slot: Option<ImageUploadSlotView>,
}
