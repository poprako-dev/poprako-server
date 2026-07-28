//! Val DTOs for the comic domain.

//! Data transfer objects for comic use cases — input parameters and
//! presentation-ready values for the comic aggregate.
//!
//! Timestamps are converted to Unix milliseconds for JSON serialisation.
//! Cover URLs are resolved from object-storage keys via [`ImagePool`].
//!
//! [`ImagePool`]: crate::part::image::ImagePool

use futures::future::OptionFuture;
use serde::Serialize;

use poprako_util::time::ToUnixMilli;

use crate::data::val::team::TeamInfoVal;
use crate::data::val::user::UserInfoVal;
use crate::data::val::workset::WorksetInfoVal;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::model::read::proj::comic::ComicInfo;
use crate::part::image::ImagePool;
use crate::result::{BaseRest, accept};

pub use crate::data::val::image::ReserveImageVal as ReserveComicCoverVal;

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
/// [`ComicInfo`]: crate::model::read::proj::comic::ComicInfo
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ComicInfoVal {
    //
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
    ) -> BaseRest<Self>
    where
        P: ImagePool,
    {
        let cover_key = match (model.is_cover_uploaded, &model.cover_key) {
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

/// Return value from a successful comic creation.
///
/// Includes the IDs of both the new comic and its auto-created first chapter.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateComicVal {
    //
    /// Newly created comic identifier.
    pub id: String,
    /// Identifier of the auto-created first chapter.
    pub chapter_id: String,
}
