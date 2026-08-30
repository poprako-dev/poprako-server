//! View DTOs for the user domain.

#[cfg(test)]
mod tests;

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::model::read::proj::user::UserInfo;

/// Presentation-ready user profile information.
///
/// Converts raw [`UserInfo`] timestamps to Unix milliseconds and accepts an
/// avatar origin and thumbnail URLs already resolved by the use-case layer.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UserInfoView {
    //
    /// Unique user identifier.
    pub id: String,

    /// User display nickname.
    pub nickname: String,
    /// Unique qualified identifier used for login lookup.
    pub qid: String,

    /// Resolved signed download URL for the avatar image, or [`None`] if
    /// no avatar has been uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    /// Resolved signed download URL for the avatar thumbnail, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_thumbnail_url: Option<String>,
    /// Whether this user has super-admin privileges.
    pub is_sadmin: bool,
    /// Timestamp of the user's most recent activity, in milliseconds since Unix epoch.
    pub last_active_at: i64,

    /// Timestamp of user account creation, in milliseconds since Unix epoch.
    pub created_at: i64,
    /// Timestamp of the last profile update, in milliseconds since Unix epoch.
    pub updated_at: i64,
}

impl UserInfoView {
    /// Converts a [`UserInfo`] into a presentation-ready value.
    ///
    /// Accepts resolved avatar origin/thumbnail URLs and converts timestamps from
    /// [`OffsetDateTime`] to Unix milliseconds.
    ///
    /// [`OffsetDateTime`]: time::OffsetDateTime
    /// [`UserInfo`]: crate::model::read::proj::user::UserInfo
    pub fn from_model(
        model: UserInfo,
        avatar_url: Option<String>,
        avatar_thumbnail_url: Option<String>,
    ) -> Self {
        //
        Self {
            id: model.id,
            nickname: model.nickname,
            qid: model.qid,
            avatar_url,
            avatar_thumbnail_url,
            is_sadmin: model.is_sadmin,
            last_active_at: model.last_active_at.to_unix_milli(),
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        }
    }
}
