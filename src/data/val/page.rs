//! Val DTOs for the page domain.

//! Data transfer objects for page use cases.

use serde::Serialize;

use crate::data::view::image::ImageUploadSlotView;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli;

use crate::model::read::proj::page::PageInfo;
use crate::part::image::ImagePool;
use crate::result::{BaseRest, accept};
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
    /// Materialize page image URLs when uploaded, then render API payload fields.
    /// Converts a [`PageInfo`] into a presentation-ready value.
    pub async fn from_model<P>(
        image_pool: &P,
        model: PageInfo,
    ) -> BaseRest<Self>
    where
        P: ImagePool,
    {
        let (image_url, image_thumbnail_url) =
            match (model.is_image_uploaded, &model.image_key) {
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

/// Return value from successful chapter page reservations.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveChapterPagesVal {
    /// Reserved pages with upload targets.
    pub pages: Vec<ReservedPageVal>,
}

/// One reserved page upload target.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReservedPageVal {
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
    pub slot: Option<ImageUploadSlotView>,
}
