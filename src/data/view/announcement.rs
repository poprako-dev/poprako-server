//! View DTOs for the announcement domain.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::data::view::user::UserInfoView;
use crate::model::read::proj::announcement::AnnouncementInfo;

/// Presentation-ready team announcement information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AnnouncementInfoView {
    /// Unique identifier.
    pub id: String,

    /// Owning team identifier.
    pub team_id: String,
    /// Authoring user identifier.
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Authoring user information, omitted when not requested.
    pub user: Option<UserInfoView>,

    /// Announcement title.
    pub title: String,
    /// Announcement body content.
    pub content: String,

    /// Timestamp of creation in Unix milliseconds.
    pub created_at: i64,
}

impl AnnouncementInfoView {
    /// Build the response object from a model row and resolved optional user.
    /// Converts an announcement model into a presentation value.
    pub fn from_model(
        model: AnnouncementInfo,
        user: Option<UserInfoView>,
    ) -> Self {
        //
        Self {
            id: model.id,
            team_id: model.team_id,
            user_id: model.user_id,
            user,
            title: model.title,
            content: model.content,
            created_at: model.created_at.to_unix_milli(),
        }
    }
}
