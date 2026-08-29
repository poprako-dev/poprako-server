//! View DTOs for the page domain.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_obj_dept::model::meta::ObjMeta;
use poprako_util::time::ToUnixMilli as _;

use crate::model::read::proj::page::PageInfo;
use crate::value::image::{ImageExt, ImageHash};

/// Presentation-ready page information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageInfoView {
    /// Unique page identifier.
    pub id: String,

    /// Owning chapter identifier.
    pub chapter_id: String,
    /// Ordinal position within the chapter.
    pub index: usize,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Presigned download URL for the full image, if uploaded.
    pub image_url: Option<String>,
    /// Content hash of the page image, if one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_hash: Option<ImageHash>,
    /// File format, if one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
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

impl PageInfoView {
    /// Materialize page image URLs when uploaded, then render API payload fields.
    /// Converts a [`PageInfo`] into a presentation-ready value.
    pub fn from_model(
        model: PageInfo,
        obj_meta: Option<&ObjMeta>,
        image_url: Option<String>,
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
            image_url,
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
