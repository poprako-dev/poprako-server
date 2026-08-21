//! Image-upload configuration.

use anyhow::bail;
use serde::Deserialize;

use crate::value::image::ImageKind;

/// Runtime byte limits for uploaded images.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct ImageConfig {
    //
    /// Maximum byte length for a user avatar upload.
    pub user_avatar_limit: u64,

    /// Maximum byte length for a team avatar upload.
    pub team_avatar_limit: u64,

    /// Maximum byte length for a comic cover upload.
    pub comic_cover_limit: u64,

    /// Maximum byte length for a chapter page image upload.
    pub page_image_limit: u64,
}

impl ImageConfig {
    /// Validates that every configured byte limit permits a non-empty upload.
    pub fn validate(&self) -> anyhow::Result<()> {
        //
        for (field_name, byte_limit) in [
            ("user_avatar_limit", self.user_avatar_limit),
            ("team_avatar_limit", self.team_avatar_limit),
            ("comic_cover_limit", self.comic_cover_limit),
            ("page_image_limit", self.page_image_limit),
        ] {
            //
            if byte_limit == 0 {
                //
                bail!(
                    "[ImageConfig::validate] {} must be greater than zero",
                    field_name,
                );
            }
        }

        Ok(())
    }

    /// Returns the configured byte limit for an image kind.
    pub fn byte_limit_for(&self, image_kind: ImageKind) -> u64 {
        //
        match image_kind {
            //
            ImageKind::UserAvatar => self.user_avatar_limit,

            ImageKind::TeamAvatar => self.team_avatar_limit,

            ImageKind::ComicCover => self.comic_cover_limit,

            ImageKind::PageImage => self.page_image_limit,
        }
    }
}
