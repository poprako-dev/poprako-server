//! Data transfer objects for page use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli;

use crate::data::image::ImageUploadSlotVal;
use crate::model::page::{PageImageSpec, PageInfo};
use crate::part::image::ImagePool;
use crate::result::{BaseResult, accept};
use crate::value::image::{ImageExt, ImageHash};

#[cfg(test)]
mod tests;

/// Presentation-ready page information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageInfoVal {
    //
    /// Unique page identifier.
    pub id: String,

    /// Owning chapter identifier.
    pub chapter_id: String,
    /// Ordinal position within the chapter.
    pub index: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Presigned download URL for the full image, if uploaded.
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Presigned download URL for the thumbnail, if uploaded.
    pub image_thumbnail_url: Option<String>,

    /// Content hash of the page image.
    pub image_hash: ImageHash,
    /// File format.
    pub ext: ImageExt,

    /// Total number of translation units on this page.
    pub total_unit_count: i32,
    /// Number of translated units on this page.
    pub translated_unit_count: i32,
    /// Number of proofread units on this page.
    pub proofread_unit_count: i32,

    /// Timestamp of creation, in Unix milliseconds.
    pub created_at: i64,
    /// Timestamp of last update, in Unix milliseconds.
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
            ext: model.image_ext,
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
    //
    /// Target chapter identifier.
    pub chapter_id: String,
    /// Page images to reserve for the chapter.
    pub pages: Vec<PageImageParams>,
}

/// One page image in a complete chapter manifest.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageImageParams {
    //
    /// Existing page identifier, if updating an existing page.
    pub page_id: Option<String>,
    /// Content hash of the page image.
    pub image_hash: ImageHash,
    /// Size used to constrain a newly requested upload slot.
    ///
    /// Existing manifest entries may omit this when no upload slot is needed.
    pub new_byte_len: Option<u64>,
    /// File format.
    pub ext: ImageExt,
}

impl From<PageImageParams> for PageImageSpec {
    fn from(params: PageImageParams) -> Self {
        Self {
            page_id: params.page_id,
            image_hash: params.image_hash,
            new_byte_len: params.new_byte_len,
            ext: params.ext,
        }
    }
}

/// Return value from successful chapter page reservations.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveChapterPagesPayload {
    /// Reserved pages with upload targets.
    pub pages: Vec<ReservedPagePayload>,
}

/// One reserved page upload target.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReservedPagePayload {
    //
    /// Reserved page identifier.
    pub page_id: String,

    /// Ordinal position within the chapter.
    pub index: u32,
    /// Content hash of the page image.
    pub image_hash: ImageHash,
    /// File format.
    pub ext: ImageExt,

    /// Presigned upload slot, if a new image must be uploaded.
    pub slot: Option<ImageUploadSlotVal>,
}

/// Input parameters for reserving one page image.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReservePageImageParams {
    //
    /// Content hash of the page image to reserve.
    pub image_hash: ImageHash,
    /// Size of the page image in bytes.
    pub new_byte_len: u64,
    /// File format.
    pub ext: ImageExt,
}

/// Input parameters for confirming a page image upload completed.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkPageImageUploadedParams {
    /// Image version from the reservation, used as an idempotency guard.
    pub image_version: u32,
}

/// Input parameters for listing all pages under one chapter.
#[derive(Debug)]
pub struct ListPageInfosParams {
    /// Chapter whose pages to list.
    pub chapter_id: String,
}
