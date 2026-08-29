//! View DTOs for the team domain.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::model::read::proj::team::TeamInfo;

/// Presentation-ready team profile information.
///
/// Converts raw [`TeamInfo`] timestamps to Unix milliseconds and accepts an
/// avatar URL already resolved by the use-case layer.
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
    /// Timestamp of creation, in Unix milliseconds.
    pub created_at: i64,
    /// Timestamp of last update, in Unix milliseconds.
    pub updated_at: i64,
}

impl TeamInfoView {
    /// Converts a [`TeamInfo`] into a presentation-ready value.
    ///
    /// Accepts the resolved avatar URL and converts timestamps from
    /// [`OffsetDateTime`] to Unix milliseconds.
    ///
    /// [`OffsetDateTime`]: time::OffsetDateTime
    /// [`TeamInfo`]: crate::model::read::proj::team::TeamInfo
    pub fn from_model(model: TeamInfo, avatar_url: Option<String>) -> Self {
        //
        Self {
            id: model.id,
            name: model.name,
            description: model.description,
            avatar_url,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}
