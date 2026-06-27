//! Data transfer objects for comic use cases — input parameters and
//! presentation-ready values for the comic aggregate.
//!
//! Timestamps are converted to Unix milliseconds for JSON serialisation.
//! Cover URLs are resolved from object-storage keys via [`ImagePool`].
//!
//! [`ImagePool`]: crate::part::image::ImagePool

use poprako_util::time::ToUnixMilli;

use crate::model::comic::ComicInfo;
use crate::part::image::ImagePool;
use crate::result::RootResult;
use crate::value::comic::ComicWithOpt;

/// Presentation-ready comic information.
///
/// Mirrors [`ComicInfo`] but converts timestamps to Unix milliseconds and
/// resolves the cover key to a signed download URL via [`ImagePool`] when
/// the cover has been uploaded.
///
/// Construct via [`ComicInfoVal::from_model`] — the conversion requires
/// an [`ImagePool`] instance for URL signing.
///
/// [`ComicInfo`]: crate::model::comic::ComicInfo
pub struct ComicInfoVal {
    pub id: String,

    pub workset_id: String,
    pub index: i32,

    pub title: String,
    pub author: String,
    pub description: Option<String>,
    pub is_completed: bool,

    /// Resolved signed download URL for the cover image, or [`None`] if
    /// no cover has been uploaded.
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
    ///
    /// Resolves a signed cover download URL when the cover has been uploaded
    /// and a key is present. Timestamps are converted from [`OffsetDateTime`]
    /// to Unix milliseconds.
    ///
    /// [`OffsetDateTime`]: time::OffsetDateTime
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

/// Input parameters for creating a new comic inside a workset.
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

/// Input parameters for updating a comic's title, author, and description.
///
/// Cover and workflow updates are handled by dedicated endpoints
/// ([`reserve_cover`], [`mark_completed`]).
///
/// [`reserve_cover`]: crate::usecase::comic::reserve_cover
/// [`mark_completed`]: crate::usecase::comic::mark_completed
pub struct UpdateComicInfoData {
    pub id: String,

    pub title: String,
    pub author: String,
    pub description: Option<String>,
}

/// Input parameters for listing comics within a workset.
///
/// The `with` field specifies which related entities to include in the
/// response (e.g. the pinned chapter or parent workset metadata).
pub struct ListComicInfosData {
    pub workset_id: String,
    // TODO:
    pub with: Vec<ComicWithOpt>,
}

/// Input parameters for reserving a new comic cover upload slot.
///
/// The file extension determines the object-storage key suffix. After
/// reservation the client uploads directly to the returned PUT URL.
pub struct ReserveComicCoverData {
    pub file_ext: String,
}

/// Return value from a successful cover reservation.
///
/// The client uses `put_url` to upload the cover image directly to object
/// storage. `cover_version` must be echoed back when confirming the upload.
pub struct ReserveComicCoverVal {
    pub put_url: String,
    pub cover_version: i64,
}

/// Input parameters for confirming a comic cover upload completed.
///
/// `cover_version` must match the version returned by the reservation step.
pub struct MarkComicCoverUploadedData {
    pub cover_version: i64,
}
