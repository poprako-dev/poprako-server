//! Data transfer objects for comic use cases.

use poprako_util::time::ToUnixMilli;

use crate::model::comic::ComicInfo;
use crate::part::image::ImagePool;
use crate::result::RootResult;
use crate::value::comic::ComicExtraOpt;

/// Presentation-ready comic information.
pub struct ComicInfoVal {
    pub id: String,

    pub workset_id: String,
    pub index: i32,

    pub title: String,
    pub author: String,
    pub description: Option<String>,
    pub is_completed: bool,

    pub cover_url: Option<String>,
    pub cover_version: i64,

    pub chapter_count: i32,
    pub chapter_next_index: i32,

    pub creator_id: String,
    pub last_active_at: i64,

    pub created_at: i64,
    pub updated_at: i64,
}

impl ComicInfoVal {
    /// Converts a [`ComicInfo`] into a presentation-ready value.
    pub async fn from_model<P>(image_pool: &P, model: ComicInfo) -> RootResult<Self>
    where
        P: ImagePool,
    {
        let cover_url = match (model.cover_uploaded, &model.cover_key) {
            (true, Some(key)) => image_pool.get_signed(key).await.ok(),
            _ => None,
        };

        Ok(Self {
            id: model.id,
            workset_id: model.workset_id,
            index: model.index,
            title: model.title,
            author: model.author,
            description: model.description,
            is_completed: model.is_completed,
            cover_url: cover_url.map(Into::into),
            cover_version: model.cover_version,
            chapter_count: model.chapter_count,
            chapter_next_index: model.chapter_next_index,
            creator_id: model.creator_id,
            last_active_at: model.last_active_at.to_unix_milli(),
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        })
    }
}

/// Input parameters for creating a comic.
pub struct CreateComicData {
    pub workset_id: String,

    pub title: String,
    pub author: String,
    pub description: Option<String>,
}

/// Return value from a successful comic creation.
pub struct CreateComicVal {
    pub id: String,
}

/// Input parameters for updating a comic's profile.
pub struct UpdateComicInfoData {
    pub id: String,
    pub title: String,
    pub author: String,
    pub description: Option<String>,
}

/// Input parameters for listing comics.
pub struct ListComicInfosData {
    pub workset_id: String,
    // TODO:
    pub extra_opt: Vec<ComicExtraOpt>,
}

/// Input parameters for reserving a new cover upload slot.
pub struct ReserveComicCoverData {
    pub file_ext: String,
}

/// Return value from a successful cover reservation.
pub struct ReserveComicCoverVal {
    pub put_url: String,
    pub cover_version: i64,
}

/// Input parameters for confirming a cover upload completed.
pub struct MarkComicCoverUploadedData {
    pub cover_version: i64,
}
