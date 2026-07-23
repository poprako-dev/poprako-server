//! Data transfer objects for page use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli;

use crate::model::page::PageInfo;
use crate::part::image::ImagePool;
use crate::result::{BaseResult, accept};
use crate::value::image::{ImageExtension, ImageHash};

#[cfg(test)]
mod tests;

/// Presentation-ready page information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageInfoVal {
    pub id: String,

    pub chapter_id: String,
    pub index: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_thumbnail_url: Option<String>,

    pub image_hash: ImageHash,
    pub byte_length: u64,
    pub extension: ImageExtension,

    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,

    pub created_at: i64,
    pub updated_at: i64,
}

impl PageInfoVal {
    /// Converts a [`PageInfo`] into a presentation-ready value.
    pub async fn from_model<P>(
        image_pool: &P,
        model: PageInfo,
    ) -> BaseResult<Self>
    where
        P: ImagePool,
    {
        let (image_url, image_thumbnail_url) =
            match (model.image_uploaded, &model.image_key) {
                //
                (true, Some(key)) => (
                    image_pool.gen_download_url(key).await.ok(),
                    image_pool.gen_thumbnail_download_url(key).await.ok(),
                ),

                _ => (None, None),
            };

        accept(Self {
            id: model.id,
            chapter_id: model.chapter_id,
            index: model.index,
            image_url: image_url.map(Into::into),
            image_thumbnail_url: image_thumbnail_url.map(Into::into),
            image_hash: model.image_hash,
            byte_length: model.image_byte_length,
            extension: model.image_extension,
            total_unit_count: model.total_unit_count,
            translated_unit_count: model.translated_unit_count,
            proofread_unit_count: model.proofread_unit_count,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        })
    }
}

/// Input parameters for reserving all page images of a chapter.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveChapterPagesParams {
    pub chapter_id: String,
    pub pages: Vec<PageImageParams>,
}

/// One page image in a complete chapter manifest.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageImageParams {
    pub page_id: Option<String>,
    pub image_hash: ImageHash,
    pub byte_length: u64,
    pub extension: ImageExtension,
}

/// Return value from successful chapter page reservations.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveChapterPagesPayload {
    pub pages: Vec<ReservedPagePayload>,
}

/// One reserved page upload target.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReservedPagePayload {
    pub page_id: String,
    pub index: u32,
    pub image_hash: ImageHash,
    pub byte_length: u64,
    pub extension: ImageExtension,
    pub upload: Option<PageImageUploadPayload>,
}

/// Presigned target for a pending page-image upload.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageImageUploadPayload {
    pub put_url: String,
    pub image_version: u32,
    pub headers: std::collections::BTreeMap<String, String>,
}

/// Input parameters for reserving one page image.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReservePageImageParams {
    pub image_hash: ImageHash,
    pub byte_length: u64,
    pub extension: ImageExtension,
}

/// Input parameters for confirming a page image upload completed.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkPageImageUploadedParams {
    pub image_version: u32,
}

/// Input parameters for listing all pages under one chapter.
pub struct ListPageInfosParams {
    pub chapter_id: String,
}
