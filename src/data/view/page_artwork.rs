//! Presentation of independently ordered composite artwork pages.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_obj_dept::model::meta::ObjMeta;
use poprako_util::time::ToUnixMilli as _;

use crate::model::read::proj::page_artwork::PageArtworkInfo;
use crate::value::image::{ImageExt, ImageHash};

/// Presentation-ready composite artwork page information.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[cfg_attr(test, derive(Debug))]
pub struct PageArtworkInfoView {
    /// Unique page identifier.
    pub id: String,

    /// Owning chapter identifier.
    pub chapter_id: String,
    /// Ordinal position within the chapter.
    pub index: usize,

    /// Original PSD filename or relative path.
    pub raw_ident: Option<String>,

    /// Presigned download URL for the full image, if uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// Presigned download URL for the thumbnail image, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_thumbnail_url: Option<String>,
    /// Content hash of the page image, if one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_hash: Option<ImageHash>,
    /// File format, if one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<ImageExt>,
    /// Current allocated image version.
    #[serde(rename = "image_version")]
    pub image_ver: Option<u32>,
    /// Whether the current image is available.
    pub image_uploaded: bool,

    /// Timestamp of creation, in Unix milliseconds.
    pub created_at: i64,
    /// Timestamp of last update, in Unix milliseconds.
    pub updated_at: i64,
}

impl PageArtworkInfoView {
    /// Converts a [`PageArtworkInfo`] into a presentation-ready value.
    pub fn from_model(
        model: PageArtworkInfo,
        obj_meta: Option<&ObjMeta>,
        image_url: Option<String>,
        image_thumbnail_url: Option<String>,
    ) -> Self {
        //
        let image_hash = obj_meta.and_then(|meta| {
            //
            let bytes = <[u8; 32]>::try_from(meta.hash.as_slice()).ok()?;

            Some(ImageHash::new(bytes))
        });

        let ext = obj_meta.and_then(|meta| ImageExt::parse(&meta.ext));

        Self {
            id: model.id,
            chapter_id: model.chapter_id,
            index: model.index,
            raw_ident: model.raw_ident,
            image_url,
            image_thumbnail_url,
            image_hash,
            ext,
            image_ver: obj_meta.map(|meta| meta.key.ver),
            image_uploaded: obj_meta.is_some_and(|meta| meta.is_avail),
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}
