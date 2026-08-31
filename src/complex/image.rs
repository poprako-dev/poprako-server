//! Complex domain logic for image lifecycle tracking — deletion and integrity-check ID generation for asynchronous image processing.

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use poprako_util::i18n::trl_kv;

use crate::config::image::ImageConfig;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::image::{
    ComicCoverKey, ImageExt, ImageKind, PageImageKey, TeamAvatarKey,
    UserAvatarKey,
};

/// Domain opers for image lifecycle management: generates unique identifiers for scheduled image deletion and integrity check tasks.
pub struct ImageComplex;

impl ImageComplex {
    // Number of bytes in one mebibyte.
    const BYTES_PER_MIB: u64 = 1024 * 1024;

    /// Builds the canonical page-image physical key.
    pub fn page_key(value: &PageImageKey, ver: u32) -> String {
        //
        format!(
            "page/chapter_{}/{}-{}.{}",
            value.chapter_id,
            value.page_id,
            ver,
            value.ext.suffix(),
        )
    }

    /// Parses one canonical page-image physical key.
    pub fn parse_page_key(value: &str) -> Option<(PageImageKey, u32)> {
        //
        let path = value.strip_prefix("page/chapter_")?;

        let (chapter_id, filename) = path.split_once('/')?;

        let (stem, ext) = filename.rsplit_once('.')?;

        let (page_id, ver) = stem.rsplit_once('-')?;

        let image_key = PageImageKey {
            chapter_id: chapter_id.to_owned(),
            page_id: page_id.to_owned(),
            ext: ImageExt::parse(ext)?,
        };

        let ver = ver.parse().ok()?;

        (Self::page_key(&image_key, ver) == value).then_some((image_key, ver))
    }

    /// Builds the canonical user-avatar physical key.
    pub fn user_avatar_key(value: &UserAvatarKey, ver: u32) -> String {
        Self::flat_key("user_avatar", &value.user_id, ver, value.ext)
    }

    /// Parses one canonical user-avatar physical key.
    pub fn parse_user_avatar_key(value: &str) -> Option<(UserAvatarKey, u32)> {
        //
        let (id, ext, ver) = Self::parse_flat_key("user_avatar", value)?;

        Some((UserAvatarKey { user_id: id, ext }, ver))
    }

    /// Builds the canonical team-avatar physical key.
    pub fn team_avatar_key(value: &TeamAvatarKey, ver: u32) -> String {
        Self::flat_key("team_avatar", &value.team_id, ver, value.ext)
    }

    /// Parses one canonical team-avatar physical key.
    pub fn parse_team_avatar_key(value: &str) -> Option<(TeamAvatarKey, u32)> {
        //
        let (id, ext, ver) = Self::parse_flat_key("team_avatar", value)?;

        Some((TeamAvatarKey { team_id: id, ext }, ver))
    }

    /// Builds the canonical comic-cover physical key.
    pub fn comic_cover_key(value: &ComicCoverKey, ver: u32) -> String {
        Self::flat_key("comic_cover", &value.comic_id, ver, value.ext)
    }

    /// Parses one canonical comic-cover physical key.
    pub fn parse_comic_cover_key(value: &str) -> Option<(ComicCoverKey, u32)> {
        //
        let (id, ext, ver) = Self::parse_flat_key("comic_cover", value)?;

        Some((ComicCoverKey { comic_id: id, ext }, ver))
    }

    /// Validates the content length against the per-kind upper bound.
    ///
    pub fn ensure_byte_length(
        image_config: &ImageConfig,
        byte_length: u64,
        kind: ImageKind,
    ) -> BaseRest<()> {
        //
        let max_mib = image_config.limit_for(kind);

        let max_length = max_mib.saturating_mul(Self::BYTES_PER_MIB);

        if !(1..=max_length).contains(&byte_length) {
            //
            return Err(Self::invalid_byte_length_rejection(
                image_config,
                byte_length,
                kind,
            ));
        }

        accept(())
    }

    /// Builds the client-correctable rejection for a missing or invalid image length.
    pub fn invalid_byte_length_rejection(
        image_config: &ImageConfig,
        byte_length: u64,
        kind: ImageKind,
    ) -> BaseError {
        //
        let max_mib = image_config.limit_for(kind);

        let args = HashMap::from([
            ("min_bytes".into(), 1_u64.into()),
            ("max_mib".into(), max_mib.into()),
        ]);

        let err_message = trl_kv("error-invalid-image-byte-length", &args);

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            byte_length,
            max_length = max_mib.saturating_mul(Self::BYTES_PER_MIB),
            image_kind = ?kind,
            "expected error: image byte length is invalid",
        );

        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        }
    }

    // Builds one canonical non-page physical key.
    fn flat_key(prefix: &str, id: &str, ver: u32, ext: ImageExt) -> String {
        format!("{}/{}-{}.{}", prefix, id, ver, ext.suffix())
    }

    // Parses one canonical non-page physical key.
    fn parse_flat_key(
        prefix: &str,
        value: &str,
    ) -> Option<(String, ImageExt, u32)> {
        //
        let filename = value.strip_prefix(prefix)?.strip_prefix('/')?;

        let (stem, ext) = filename.rsplit_once('.')?;

        let (id, ver) = stem.rsplit_once('-')?;

        let ext = ImageExt::parse(ext)?;

        let ver = ver.parse().ok()?;

        (Self::flat_key(prefix, id, ver, ext) == value).then_some((
            id.to_owned(),
            ext,
            ver,
        ))
    }
}
