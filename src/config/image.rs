//! Image-upload configuration.

use anyhow::bail;
use serde::Deserialize;

use crate::value::image::ImageKind;

/// Runtime MiB limits for uploaded images.
#[derive(Clone, Copy, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ImageConfig {
    /// Maximum MiB length for a user avatar upload.
    pub user_avatar_limit_mib: u64,

    /// Maximum MiB length for a team avatar upload.
    pub team_avatar_limit_mib: u64,

    /// Maximum MiB length for a comic cover upload.
    pub comic_cover_limit_mib: u64,

    /// Maximum MiB length for a chapter page image upload.
    pub page_source_image_limit_mib: u64,

    /// Maximum MiB length for a finished page image upload.
    pub page_artwork_image_limit_mib: u64,
}

impl ImageConfig {
    /// Validates that every configured MiB limit permits a non-empty upload.
    ///
    /// # Errors
    ///
    /// Returns an error when any configured limit is zero.
    pub fn validate(&self) -> anyhow::Result<()> {
        //
        for (field_name, mib_limit) in [
            ("user_avatar_limit_mib", self.user_avatar_limit_mib),
            ("team_avatar_limit_mib", self.team_avatar_limit_mib),
            ("comic_cover_limit_mib", self.comic_cover_limit_mib),
            (
                "page_source_image_limit_mib",
                self.page_source_image_limit_mib,
            ),
            (
                "page_artwork_image_limit_mib",
                self.page_artwork_image_limit_mib,
            ),
        ] {
            //
            if mib_limit == 0 {
                //
                bail!(
                    "[ImageConfig::\
                        validate] {field_name} must be greater than zero",
                );
            }
        }

        Ok(())
    }

    /// Returns the configured MiB limit for an image kind.
    #[must_use]
    pub const fn limit_for(&self, image_kind: ImageKind) -> u64 {
        //
        match image_kind {
            //
            ImageKind::UserAvatar => self.user_avatar_limit_mib,

            ImageKind::TeamAvatar => self.team_avatar_limit_mib,

            ImageKind::ComicCover => self.comic_cover_limit_mib,

            ImageKind::PageSourceImage => self.page_source_image_limit_mib,

            ImageKind::PageArtworkImage => self.page_artwork_image_limit_mib,
        }
    }
}

impl Default for ImageConfig {
    // Supplies the default runtime policy.
    fn default() -> Self {
        Self {
            user_avatar_limit_mib: 1,
            team_avatar_limit_mib: 1,
            comic_cover_limit_mib: 2,
            page_source_image_limit_mib: 25,
            page_artwork_image_limit_mib: 25,
        }
    }
}
