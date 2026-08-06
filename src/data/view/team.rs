//! View DTOs for the team domain.

use serde::Serialize;

use poprako_util::time::ToUnixMilli as _;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::model::read::proj::team::TeamInfo;
use crate::part::image::ImagePool;
use crate::result::{BaseRest, accept};

/// Presentation-ready team profile information.
///
/// Converts the raw [`TeamInfo`] timestamps to Unix milliseconds and
/// resolves the avatar key to a signed download URL via [`ImagePool`] when
/// the avatar has been uploaded.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct TeamInfoView {
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

impl TeamInfoView {
    /// Converts a [`TeamInfo`] into a presentation-ready value.
    ///
    /// Resolves a signed avatar download URL when the avatar has
    /// been uploaded and a key is present. Timestamps are converted
    /// from [`OffsetDateTime`] to Unix milliseconds.
    ///
    /// [`OffsetDateTime`]: time::OffsetDateTime
    /// [`TeamInfo`]: crate::model::read::proj::team::TeamInfo
    pub async fn from_model<P>(
        image_pool: &P,
        model: TeamInfo,
    ) -> BaseRest<Self>
    where
        P: ImagePool,
    {
        let (avatar_url, avatar_thumbnail_url) =
            match (model.is_avatar_uploaded, &model.avatar_key) {
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
