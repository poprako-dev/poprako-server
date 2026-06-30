//! Data transfer objects for page use cases.

use poprako_macro::Paginate;
use poprako_util::time::ToUnixMilli;

use crate::model::page::PageInfo;
use crate::part::image::ImagePool;
use crate::result::RootResult;

/// Presentation-ready page information.
pub struct PageInfoVal {
    pub id: String,

    pub chapter_id: String,
    pub index: i32,

    pub image_url: Option<String>,
    pub image_uploaded: bool,

    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,

    pub created_at: i64,
    pub updated_at: i64,
}

impl PageInfoVal {
    /// Converts a [`PageInfo`] into a presentation-ready value.
    pub async fn from_model<P>(image_pool: &P, model: PageInfo) -> RootResult<Self>
    where
        P: ImagePool,
    {
        let image_url = match (model.image_uploaded, &model.image_key) {
            (true, Some(key)) => image_pool.get_signed(key).await.ok(),
            _ => None,
        };

        Ok(Self {
            id: model.id,
            chapter_id: model.chapter_id,
            index: model.index,
            image_url: image_url.map(Into::into),
            image_uploaded: model.image_uploaded,
            total_unit_count: model.total_unit_count,
            translated_unit_count: model.translated_unit_count,
            proofread_unit_count: model.proofread_unit_count,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        })
    }
}

/// Input parameters for reserving all page images of a chapter.
pub struct ReserveChapterPagesData {
    pub chapter_id: String,
    pub page_count: i32,
    pub file_ext: String,
}

/// Return value from successful chapter page reservations.
pub struct ReserveChapterPagesVal {
    pub creations: Vec<PageCreationVal>,
}

/// One reserved page upload target.
pub struct PageCreationVal {
    pub page_id: String,
    pub put_url: String,
    pub image_version: i64,
}

/// Input parameters for reserving one page image.
pub struct ReservePageImageData {
    pub file_ext: String,
}

/// Return value from a successful single-page image reservation.
pub struct ReservePageImageVal {
    pub page_id: String,
    pub put_url: String,
    pub image_version: i64,
}

/// Input parameters for confirming a page image upload completed.
pub struct MarkPageImageUploadedData {
    pub image_version: i64,
}

/// Input parameters for listing pages under one chapter.
#[Paginate]
pub struct ListPageInfosData {
    pub chapter_id: String,
}
