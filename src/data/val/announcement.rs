//! Val DTOs for the announcement domain.

//! Data transfer objects for announcement use cases.

use serde::Serialize;

use crate::data::val::user::UserInfoVal;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use futures::future::OptionFuture;

use poprako_util::time::ToUnixMilli;

use crate::model::read::proj::announcement::AnnouncementInfo;
use crate::part::image::ImagePool;
use crate::result::{BaseRest, accept};

/// Presentation-ready team announcement information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AnnouncementInfoVal {
    //
    /// Unique identifier.
    pub id: String,

    /// Owning team identifier.
    pub team_id: String,
    /// Authoring user identifier.
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Authoring user information, omitted when not requested.
    pub user: Option<UserInfoVal>,

    /// Announcement title.
    pub title: String,
    /// Announcement body content.
    pub content: String,

    /// Timestamp of creation in Unix milliseconds.
    pub created_at: i64,
}

impl AnnouncementInfoVal {
    /// Build the response object from a model row and resolved optional user.
    /// Converts an announcement model into a presentation value.
    pub async fn from_model<P>(
        image_pool: &P,
        model: AnnouncementInfo,
    ) -> BaseRest<Self>
    where
        P: ImagePool,
    {
        accept(Self {
            id: model.id,
            team_id: model.team_id,
            user_id: model.user_id,
            user: OptionFuture::from(model.user.map(|user_info| {
                UserInfoVal::from_model(image_pool, user_info)
            }))
            .await
            .transpose()?,
            title: model.title,
            content: model.content,
            created_at: model.created_at.to_unix_milli(),
        })
    }
}

/// Return value from creating an announcement.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateAnnouncementVal {
    /// Identifier of the created announcement.
    pub id: String,
}
