//! Data transfer objects for announcement use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use futures::future::OptionFuture;

use poprako_util::time::ToUnixMilli;

use crate::data::user::UserInfoVal;
use crate::model::announcement::{AnnouncementInfo, AnnouncementListSpec};
use crate::part::image::ImagePool;
use crate::result::{BaseRest, accept};
use crate::value::announcement::AnnouncementInclOpt;

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

/// Input parameters for listing announcements.
///
/// `incl` embeds related rows into each item.
///
/// Example: `/api/v1/announcements?team_id=t_1&incl=user&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListAnnouncementInfosParams {
    //
    /// Parent team whose announcements to list.
    pub team_id: String,

    /// Related rows to embed. Repeatable. Values: `user`.
    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<AnnouncementInclOpt>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}

impl From<ListAnnouncementInfosParams> for AnnouncementListSpec {
    // Map listing parameters directly to the repository spec.
    fn from(params: ListAnnouncementInfosParams) -> Self {
        Self {
            team_id: params.team_id,
            incl_opt: params.incl_opt,
            offset: params.offset,
            limit: params.limit,
        }
    }
}

/// Input parameters for creating an announcement.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateAnnouncementParams {
    //
    /// Target team identifier.
    pub team_id: String,

    /// Announcement title.
    pub title: String,
    /// Announcement body content.
    pub content: String,
}

/// Return value from creating an announcement.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateAnnouncementPayload {
    /// Identifier of the created announcement.
    pub id: String,
}
