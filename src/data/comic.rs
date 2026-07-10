//! Data transfer objects for comic use cases — input parameters and
//! presentation-ready values for the comic aggregate.
//!
//! Timestamps are converted to Unix milliseconds for JSON serialisation.
//! Cover URLs are resolved from object-storage keys via [`ImagePool`].
//!
//! [`ImagePool`]: crate::part::image::ImagePool

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::{IntoParams, ToSchema};

use poprako_macro::Paginate;
use poprako_util::i18n::trl;
use poprako_util::time::ToUnixMilli;

use crate::data::chapter::ChapterInfoVal;
use crate::data::team::TeamInfoVal;
use crate::data::user::UserInfoVal;
use crate::data::workset::WorksetInfoVal;
use crate::model::comic::{ComicInfo, ComicListKind, ComicListSpec};
use crate::part::image::ImagePool;
use crate::result::{ExpectedVariant, RegularError, RegularResult};
use crate::value::chapter::StageMask;
use crate::value::comic::{ComicInclOpt, ComicWithOpt};

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
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
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

    pub chapter_count: i32,
    pub chapter_next_index: i32,

    pub creator_id: String,

    pub workset: Option<WorksetInfoVal>,
    pub team: Option<TeamInfoVal>,
    pub creator: Option<UserInfoVal>,

    #[cfg_attr(feature = "swagger-ui", schema(no_recursion))]
    pub pinned_chapter: Option<ChapterInfoVal>,

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
    pub async fn from_model<P>(
        image_pool: &P,
        model: ComicInfo,
        pinned_chapter: Option<ChapterInfoVal>,
    ) -> RegularResult<Self>
    where
        P: ImagePool,
    {
        let cover_url = match (model.cover_uploaded, &model.cover_key) {
            (true, Some(key)) => image_pool.get_signed(key).await.ok(),
            _ => None,
        };

        let workset = model.workset.map(WorksetInfoVal::from);
        let team = match model.team {
            Some(team_info) => {
                Some(TeamInfoVal::from_model(image_pool, team_info).await?)
            }
            None => None,
        };
        let creator = match model.creator {
            Some(user_info) => {
                Some(UserInfoVal::from_model(image_pool, user_info).await?)
            }
            None => None,
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
            chapter_count: model.chapter_count,
            chapter_next_index: model.chapter_next_index,
            creator_id: model.creator_id,
            workset,
            team,
            creator,
            pinned_chapter,
            last_active_at: model.last_active_at.to_unix_milli(),
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        })
    }
}

/// Input parameters for creating a new comic inside a workset.
///
/// The first chapter is created atomically with the comic. Its subtitle
/// can be customised via `first_chapter_subtitle`; when absent, a
/// locale-aware default (e.g. "Ch. 1") is generated.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateComicData {
    pub workset_id: String,

    pub title: String,
    pub author: String,
    pub description: Option<String>,

    /// Optional subtitle for the first chapter created alongside the comic.
    pub first_chapter_subtitle: Option<String>,
}

/// Return value from a successful comic creation.
///
/// Includes the IDs of both the new comic and its auto-created first chapter.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateComicVal {
    pub id: String,
    pub chapter_id: String,
}

/// Input parameters for updating a comic's title, author, and description.
///
/// Cover and workflow updates are handled by dedicated endpoints
/// ([`reserve_cover`], [`mark_archived`]).
///
/// [`reserve_cover`]: crate::usecase::comic::reserve_cover
/// [`mark_archived`]: crate::usecase::comic::mark_archived
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UpdateComicInfoData {
    pub id: String,

    pub title: String,
    pub author: String,
    pub description: Option<String>,
}

/// Input parameters for listing comics within a workset.
#[Paginate]
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct ListComicInfosData {
    pub workset_id: String,

    pub fuzzy_title: Option<String>,
    pub is_completed: Option<bool>,
    pub stages: Option<u32>,

    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub incl_opt: Vec<ComicInclOpt>,

    #[serde(
        default,
        rename = "with",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub with_opt: Vec<ComicWithOpt>,
}

impl TryFrom<ListComicInfosData> for ComicListSpec {
    type Error = RegularError;

    fn try_from(data: ListComicInfosData) -> RegularResult<Self> {
        let stages = data.stages.map(StageMask::try_filter_from).transpose()?;

        let kind = match (data.is_completed, stages) {
            (Some(true), Some(_)) => {
                return Err(RegularError::Expected {
                    variant: ExpectedVariant::Args,
                    message: trl("error-invalid-stage-phase"),
                });
            }
            (Some(true), None) => ComicListKind::Completed,
            (Some(false), stages) => ComicListKind::Active { stages },
            (None, Some(stages)) => ComicListKind::Active {
                stages: Some(stages),
            },
            (None, None) => ComicListKind::All,
        };

        Ok(Self {
            workset_id: data.workset_id,
            fuzzy_title: data.fuzzy_title,
            kind,
            incl_opt: data.incl_opt,
            offset: data.offset,
            limit: data.limit,
        })
    }
}

/// Input parameters for reserving a new comic cover upload slot.
///
/// The file extension determines the object-storage key suffix. After
/// reservation the client uploads directly to the returned PUT URL.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ReserveComicCoverData {
    pub file_ext: String,
}

/// Return value from a successful cover reservation.
///
/// The client uses `put_url` to upload the cover image directly to object
/// storage. `cover_version` must be echoed back when confirming the upload.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ReserveComicCoverVal {
    pub put_url: String,
    pub cover_version: i64,
}

/// Input parameters for confirming a comic cover upload completed.
///
/// `cover_version` must match the version returned by the reservation step.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct MarkComicCoverUploadedData {
    pub cover_version: i64,
}

/// Input parameters for marking a comic archived.
///
/// `comic_id` must match the path parameter — a mismatch is rejected with `422`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct MarkComicArchivedData {
    pub comic_id: String,
}
