//! Complex domain logic for image lifecycle tracking — deletion and integrity-check ID generation for asynchronous image processing.

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use poprako_util::i18n::trl_kv;

use crate::config::image::ImageConfig;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::value::business_id::BusinessIdKind;
use crate::value::image::{
    ComicCoverKey, ImageExt, ImageKind, PageSourceImageKey, TeamAvatarKey,
    UserAvatarKey,
};

// Number of bytes in one mebibyte.
const BYTES_PER_MIB: u64 = 1024 * 1024;

/// Builds the canonical source image key from complete business identities.
pub fn page_source_key(value: &PageSourceImageKey, ver: u32) -> String {
    //
    format!(
        "page_source/{}/{}/{}.{}",
        value.chapter_id,
        value.page_source_id,
        ver,
        value.ext.suffix()
    )
}

/// Parses a source image key with exact domain and generation validation.
pub fn parse_page_source_key(value: &str) -> Option<(PageSourceImageKey, u32)> {
    //
    let path = value.strip_prefix("page_source/")?;

    let (chapter_id, path) = path.split_once('/')?;

    let (page_source_id, gen_no) = path.split_once('/')?;

    let (ver, ext) = gen_no.split_once('.')?;

    let dom = PageSourceImageKey {
        chapter_id: chapter_id.into(),
        page_source_id: page_source_id.into(),
        ext: ImageExt::parse(ext)?,
    };

    let ver = ver.parse().ok()?;

    (BusinessIdKind::Chapter.is_valid_id(chapter_id)
        && BusinessIdKind::PageSource.is_valid_id(page_source_id)
        && page_source_key(&dom, ver) == value)
        .then_some((dom, ver))
}

/// Builds the canonical user-avatar physical key.
pub fn user_avatar_key(value: &UserAvatarKey, ver: u32) -> String {
    flat_key("user_avatar", &value.user_id, ver, value.ext)
}

/// Parses one canonical user-avatar physical key.
pub fn parse_user_avatar_key(value: &str) -> Option<(UserAvatarKey, u32)> {
    //
    let (id, ext, ver) = parse_flat_key("user_avatar", value)?;

    Some((UserAvatarKey { user_id: id, ext }, ver))
}

/// Builds the canonical team-avatar physical key.
pub fn team_avatar_key(value: &TeamAvatarKey, ver: u32) -> String {
    flat_key("team_avatar", &value.team_id, ver, value.ext)
}

/// Parses one canonical team-avatar physical key.
pub fn parse_team_avatar_key(value: &str) -> Option<(TeamAvatarKey, u32)> {
    //
    let (id, ext, ver) = parse_flat_key("team_avatar", value)?;

    Some((TeamAvatarKey { team_id: id, ext }, ver))
}

/// Builds the canonical comic-cover physical key.
pub fn comic_cover_key(value: &ComicCoverKey, ver: u32) -> String {
    flat_key("comic_cover", &value.comic_id, ver, value.ext)
}

/// Parses one canonical comic-cover physical key.
pub fn parse_comic_cover_key(value: &str) -> Option<(ComicCoverKey, u32)> {
    //
    let (id, ext, ver) = parse_flat_key("comic_cover", value)?;

    Some((ComicCoverKey { comic_id: id, ext }, ver))
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

    expected_error! {
        ExpectedVariant::Args,
        message: trl_kv("error-invalid-image-byte-length", &args);
        byte_length,
        max_length = max_mib.saturating_mul(BYTES_PER_MIB),
        image_kind = ?kind,
        "image byte length is invalid",
    }
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

    let max_length = max_mib.saturating_mul(BYTES_PER_MIB);

    if !(1..=max_length).contains(&byte_length) {
        //
        return Err(invalid_byte_length_rejection(
            image_config,
            byte_length,
            kind,
        ));
    }

    accept!(())
}

// Builds one canonical non-page physical key.
fn flat_key(prefix: &str, id: &str, ver: u32, ext: ImageExt) -> String {
    format!("{}/{}/{}.{}", prefix, id, ver, ext.suffix())
}

// Parses one canonical non-page physical key.
fn parse_flat_key(
    prefix: &str,
    value: &str,
) -> Option<(String, ImageExt, u32)> {
    //
    let path = value.strip_prefix(prefix)?.strip_prefix('/')?;

    let (id, filename) = path.split_once('/')?;

    let (ver, ext) = filename.split_once('.')?;

    let kind = match prefix {
        //
        "user_avatar" => BusinessIdKind::User,

        "team_avatar" => BusinessIdKind::Team,

        "comic_cover" => BusinessIdKind::Comic,

        _ => return None,
    };

    if !kind.is_valid_id(id) {
        return None;
    }

    let ext = ImageExt::parse(ext)?;

    let ver = ver.parse().ok()?;

    (flat_key(prefix, id, ver, ext) == value).then_some((
        id.to_owned(),
        ext,
        ver,
    ))
}
