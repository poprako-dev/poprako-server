//! View DTOs for the page domain.
use poprako_obj_dept::key::ObjKind;
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::url::ObjUrls;
use poprako_util::time::ToUnixMilli as _;

use crate::model::read::proj::page_source::PageSourceInfo;
use crate::value::image::{ImageExt, ImageHash};

/// Presentation-ready page information.
#[cfg_attr(test, derive(Debug))]
pub struct PageSourceInfoView {
    /// Unique page identifier.
    pub id: String,

    /// Owning chapter identifier.
    pub chapter_id: String,

    /// Ordinal position within the chapter.
    pub index: usize,

    /// Complete original source filename, when supplied.
    pub raw_ident: Option<String>,

    /// Current allocated image generation.
    pub image_ver: Option<u32>,
    /// Whether the current generation is available to readers.
    pub image_uploaded: bool,

    /// Presigned download URL for the full image, if uploaded.
    pub image_url: Option<String>,
    /// Download URL for the optimized image, if available.
    pub image_optimized_url: Option<String>,
    /// Presigned download URL for the thumbnail image, if available.
    pub image_thumbnail_url: Option<String>,
    /// Content hash of the page image, if one exists.
    pub image_hash: Option<ImageHash>,
    /// File format, if one exists.
    pub ext: Option<ImageExt>,

    /// Total number of translation units on this page.
    pub total_unit_count: usize,
    /// Number of translated units on this page.
    pub translated_unit_count: usize,
    /// Number of proofread units on this page.
    pub proofread_unit_count: usize,

    /// Timestamp of creation, in Unix milliseconds.
    pub created_at: i64,
    /// Timestamp of last update, in Unix milliseconds.
    pub updated_at: i64,
}

impl PageSourceInfoView {
    /// Converts a [`PageSourceInfo`] into a presentation-ready value.
    pub fn from_model<K>(
        model: PageSourceInfo,
        obj_meta: Option<&ObjMeta<K>>,
        raw_ident: Option<String>,
        image_urls: Option<&ObjUrls>,
    ) -> Self
    where
        K: ObjKind<Ext = ImageExt>,
    {
        //
        let image_hash = obj_meta.and_then(|meta| {
            //
            let bytes = <[u8; 32]>::try_from(meta.hash.as_slice()).ok()?;

            Some(ImageHash::new(bytes))
        });

        let ext = obj_meta.map(|meta| meta.ext);

        Self {
            id: model.id,
            chapter_id: model.chapter_id,
            index: model.index,
            raw_ident,
            image_ver: obj_meta.map(|meta| meta.key.ver()),
            image_uploaded: obj_meta.is_some_and(|meta| meta.is_avail),
            image_url: image_urls
                .and_then(|urls| urls.origin_url.as_ref())
                .map(ToString::to_string),
            image_optimized_url: image_urls
                .and_then(|urls| urls.optimized_url.as_ref())
                .map(ToString::to_string),
            image_thumbnail_url: image_urls
                .and_then(|urls| urls.thumbnail_url.as_ref())
                .map(ToString::to_string),
            image_hash,
            ext,
            total_unit_count: model.total_unit_count,
            translated_unit_count: model.translated_unit_count,
            proofread_unit_count: model.proofread_unit_count,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}
