//! Data transfer objects for team profile use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use poprako_util::time::ToUnixMilli;

use crate::model::team::TeamInfo;
use crate::part::image::ImagePool;
use crate::result::{BaseResult, accept};

pub use crate::data::image::{MarkImageUploadedParams as MarkTeamAvatarUploadedParams, ReserveImageParams as ReserveTeamAvatarParams, ReserveImagePayload as ReserveTeamAvatarPayload};

/// Presentation-ready team profile information.
///
/// Converts the raw [`TeamInfo`] timestamps to Unix milliseconds and
/// resolves the avatar key to a signed download URL via [`ImagePool`] when
/// the avatar has been uploaded.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct TeamInfoVal {
    /// Unique team identifier.
    pub id: String,

    /// Team display name.
    pub name: String,
    /// Team description text.
    pub description: String,

    /// Signed avatar download URL, when one has been uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// Signed avatar thumbnail download URL, when one has been uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_thumbnail_url: Option<String>,

    /// Timestamp of creation, in Unix milliseconds.
    pub created_at: i64,
    /// Timestamp of last update, in Unix milliseconds.
    pub updated_at: i64,
}

impl TeamInfoVal {
    /// Converts a [`TeamInfo`] into a presentation-ready value.
    ///
    /// Resolves a signed avatar download URL when the avatar has
    /// been uploaded and a key is present. Timestamps are converted
    /// from [`OffsetDateTime`] to Unix milliseconds.
    ///
    /// [`OffsetDateTime`]: time::OffsetDateTime
    /// [`TeamInfo`]: crate::model::team::TeamInfo
    pub async fn from_model<P>(
        image_pool: &P,
        model: TeamInfo,
    ) -> BaseResult<Self>
    where
        P: ImagePool,
    {
        let (avatar_url, avatar_thumbnail_url) =
            match (model.avatar_uploaded, &model.avatar_key) {
                //
                (true, Some(key)) => (
                    image_pool.gen_download_url(key).await.ok(),
                    image_pool.gen_thumbnail_download_url(key).await.ok(),
                ),

                _ => (None, None),
            };

        accept(Self {
            id: model.id,
            name: model.name,
            description: model.description,
            avatar_url: avatar_url.map(Into::into),
            avatar_thumbnail_url: avatar_thumbnail_url.map(Into::into),
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        })
    }
}

/// Input parameters for creating a new team.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateTeamParams {
    /// Team display name.
    pub name: String,
    /// Team description text.
    pub description: String,
}

/// Input parameters for listing teams.
///
/// Exactly one listing mode applies based on `user_id`:
/// - `user_id` omitted: list all teams (requires super-admin, otherwise
///   `403`);
/// - `user_id` present: list teams the given user has joined.
///
/// Example: `/api/v1/teams?user_id=u_123&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListTeamInfosParams {
    /// Filter to teams joined by this user. Omit to list all teams
    /// (super-admin only).
    pub user_id: Option<String>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}

/// Input parameters for updating a team's profile.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateTeamInfoParams {
    /// Team identifier.
    pub id: String,

    /// Updated team display name.
    pub name: String,
    /// Updated team description text.
    pub description: String,
}
