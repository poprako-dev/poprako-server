//! Data transfer objects for comic use cases — input parameters and
//! presentation-ready values for the comic aggregate.
//!
//! Timestamps are converted to Unix milliseconds for JSON serialisation.
//! Cover URLs are resolved from object-storage keys via [`ImagePool`].
//!
//! [`ImagePool`]: crate::part::image::ImagePool

use futures::future::OptionFuture;
use serde::{Deserialize, Serialize};
#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use poprako_util::time::ToUnixMilli;

use crate::data::team::TeamInfoVal;
use crate::data::user::UserInfoVal;
use crate::data::workset::WorksetInfoVal;
use crate::model::comic::{ComicInfo, ComicInfoListKind, ComicInfoListSpec};
use crate::part::image::ImagePool;
use crate::result::{BaseError, BaseResult, accept};
use crate::value::chapter::StageMask;
use crate::value::comic::{ComicInclOpt, ComicWithOpt};
use crate::value::role::RoleMask;

pub use crate::data::image::{
    MarkImageUploadedParams as MarkComicCoverUploadedParams,
    ReserveImageParams as ReserveComicCoverParams,
    ReserveImagePayload as ReserveComicCoverPayload,
};

#[cfg(test)]
mod tests;

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
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ComicInfoVal {
    /// Unique comic identifier.
    pub id: String,

    /// Parent workset identifier this comic belongs to.
    pub workset_id: String,
    /// Ordinal position of the comic within its workset.
    pub index: i32,

    /// Comic title.
    pub title: String,
    /// Comic author name.
    pub author: String,
    /// Optional description or synopsis of the comic content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Resolved signed download URL for the cover image, or [`None`] if
    /// no cover has been uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    /// Resolved signed download URL for the cover thumbnail, or [`None`] if
    /// no cover has been uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_thumbnail_url: Option<String>,

    /// Total number of chapters in this comic.
    pub chapter_count: i32,

    /// Identifier of the user who created the comic entry.
    pub creator_id: String,

    /// Resolved workset summary, included when the `workset` expansion option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workset: Option<WorksetInfoVal>,
    /// Resolved team summary for the owning workset, included when the `team` expansion option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<TeamInfoVal>,
    /// Resolved creator profile, included when the `creator` expansion option is requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<UserInfoVal>,

    /// Timestamp of the most recent activity on the comic, in milliseconds since Unix epoch.
    pub last_active_at: i64,

    /// Timestamp of comic creation, in milliseconds since Unix epoch.
    pub created_at: i64,
    /// Timestamp of the last comic update, in milliseconds since Unix epoch.
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
        fallback_cover_key: Option<&str>,
    ) -> BaseResult<Self>
    where
        P: ImagePool,
    {
        let cover_key = match (model.cover_uploaded, &model.cover_key) {
            //
            (true, Some(key)) => Some(key.as_str()),

            _ => fallback_cover_key,
        };

        let (cover_url, cover_thumbnail_url) = match cover_key {
            //
            Some(key) => (
                image_pool.gen_download_url(key).await.ok(),
                image_pool.gen_thumbnail_download_url(key).await.ok(),
            ),

            None => (None, None),
        };

        let workset = model.workset.map(WorksetInfoVal::from);

        accept(Self {
            id: model.id,
            workset_id: model.workset_id,
            index: model.index,
            title: model.title,
            author: model.author,
            description: model.description,
            cover_url: cover_url.map(Into::into),
            cover_thumbnail_url: cover_thumbnail_url.map(Into::into),
            chapter_count: model.chapter_count,
            creator_id: model.creator_id,
            workset,
            team: OptionFuture::from(model.team.map(|team_info| {
                TeamInfoVal::from_model(image_pool, team_info)
            }))
            .await
            .transpose()?,
            creator: OptionFuture::from(model.creator.map(|user_info| {
                UserInfoVal::from_model(image_pool, user_info)
            }))
            .await
            .transpose()?,
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
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateComicParams {
    /// Parent workset identifier.
    pub workset_id: String,

    /// Comic title.
    pub title: String,
    /// Comic author name.
    pub author: String,
    /// Optional description of the comic.
    pub description: Option<String>,

    /// Optional subtitle for the first chapter created alongside the comic.
    pub first_chapter_subtitle: Option<String>,

    /// Roles assigned to the creator on the first chapter in addition to the
    /// mandatory admin role. Every requested role must exist on the creator's
    /// team membership.
    pub preset_assignment_roles: Option<RoleMask>,
}

/// Return value from a successful comic creation.
///
/// Includes the IDs of both the new comic and its auto-created first chapter.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateComicPayload {
    /// Newly created comic identifier.
    pub id: String,
    /// Identifier of the auto-created first chapter.
    pub chapter_id: String,
}

/// Input parameters for updating a comic's title, author, and description.
///
/// Cover updates are handled by dedicated endpoints.
///
/// [`reserve_cover`]: crate::usecase::comic::reserve_cover
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateComicInfoParams {
    /// Comic identifier.
    pub id: String,

    /// Updated comic title.
    pub title: String,
    /// Updated author name.
    pub author: String,
    /// Updated description.
    pub description: Option<String>,
}

/// Input parameters for listing comics within a workset.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListComicInfosParams {
    /// Parent workset identifier.
    pub workset_id: String,

    /// Optional fuzzy title filter.
    pub fuzzy_title: Option<String>,
    /// Optional stage mask filter.
    pub stages: Option<u32>,

    /// Optional related data to include in results.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<ComicInclOpt>,

    /// Optional expansion options for the result set.
    #[serde(default, rename = "with")]
    pub with_opt: Vec<ComicWithOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}

impl TryFrom<ListComicInfosParams> for ComicInfoListSpec {
    type Error = BaseError;

    fn try_from(params: ListComicInfosParams) -> BaseResult<Self> {
        //
        let stages =
            params.stages.map(StageMask::try_filter_from).transpose()?;

        let kind = match stages {
            //
            Some(stage_mask) => ComicInfoListKind::Stages(stage_mask),

            None => ComicInfoListKind::All,
        };

        accept(Self {
            workset_id: params.workset_id,
            fuzzy_title: params.fuzzy_title,
            kind,
            incl_opt: params.incl_opt,
            offset: params.offset,
            limit: params.limit,
        })
    }
}
