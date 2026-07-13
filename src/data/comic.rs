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

use poprako_util::time::ToUnixMilli;

use crate::data::chapter::ChapterInfoVal;
use crate::data::team::TeamInfoVal;
use crate::data::user::UserInfoVal;
use crate::data::workset::WorksetInfoVal;
use crate::model::comic::{ComicInfo, ComicInfoListKind, ComicInfoListSpec};
use crate::part::image::ImagePool;
use crate::result::{RegularError, RegularResult};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Resolved signed download URL for the cover image, or [`None`] if
    /// no cover has been uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,

    pub chapter_count: i32,
    pub chapter_next_index: i32,

    pub creator_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub workset: Option<WorksetInfoVal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<TeamInfoVal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<UserInfoVal>,

    #[serde(skip_serializing_if = "Option::is_none")]
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
            //
            (true, Some(key)) => image_pool.gen_download_url(key).await.ok(),

            _ => None,
        };

        let workset = model.workset.map(WorksetInfoVal::from);

        let team = match model.team {
            //
            Some(team_info) => {
                Some(TeamInfoVal::from_model(image_pool, team_info).await?)
            }

            None => None,
        };

        let creator = match model.creator {
            //
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
pub struct CreateComicParams {
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
pub struct CreateComicPayload {
    pub id: String,
    pub chapter_id: String,
}

/// Input parameters for updating a comic's title, author, and description.
///
/// Cover updates are handled by dedicated endpoints.
///
/// [`reserve_cover`]: crate::usecase::comic::reserve_cover
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct UpdateComicInfoParams {
    pub id: String,

    pub title: String,
    pub author: String,
    pub description: Option<String>,
}

/// Input parameters for listing comics within a workset.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct ListComicInfosParams {
    pub workset_id: String,

    pub fuzzy_title: Option<String>,
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

    pub offset: u32,
    pub limit: u32,
}

impl TryFrom<ListComicInfosParams> for ComicInfoListSpec {
    type Error = RegularError;

    fn try_from(params: ListComicInfosParams) -> RegularResult<Self> {
        //
        let stages =
            params.stages.map(StageMask::try_filter_from).transpose()?;

        let kind = match stages {
            //
            Some(stage_mask) => ComicInfoListKind::Stages(stage_mask),

            None => ComicInfoListKind::All,
        };

        Ok(Self {
            workset_id: params.workset_id,
            fuzzy_title: params.fuzzy_title,
            kind,
            incl_opt: params.incl_opt,
            offset: params.offset,
            limit: params.limit,
        })
    }
}

/// Input parameters for reserving a new comic cover upload slot.
///
/// The file extension determines the object-storage key suffix. After
/// reservation the client uploads directly to the returned PUT URL.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ReserveComicCoverParams {
    pub file_ext: String,
}

/// Return value from a successful cover reservation.
///
/// The client uses `put_url` to upload the cover image directly to object
/// storage. `cover_version` must be echoed back when confirming the upload.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct ReserveComicCoverPayload {
    pub put_url: String,
    pub cover_version: u32,
}

/// Input parameters for confirming a comic cover upload completed.
///
/// `cover_version` must match the version returned by the reservation step.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct MarkComicCoverUploadedParams {
    pub cover_version: u32,
}
