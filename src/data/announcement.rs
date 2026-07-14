//! Data transfer objects for announcement use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::{IntoParams, ToSchema};

use poprako_util::time::ToUnixMilli;

use futures::future::OptionFuture;

use crate::data::user::UserInfoVal;
use crate::model::announcement::{AnnouncementInfo, AnnouncementListSpec};
use crate::part::image::ImagePool;

use crate::result::RegularResult;
use crate::value::announcement::AnnouncementInclOpt;

/// Presentation-ready team announcement information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct AnnouncementInfoVal {
    pub id: String,

    pub team_id: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfoVal>,

    pub title: String,
    pub content: String,

    pub created_at: i64,
}

impl AnnouncementInfoVal {
    /// Converts an announcement model into a presentation value.
    pub async fn from_model<P>(
        image_pool: &P,
        model: AnnouncementInfo,
    ) -> RegularResult<Self>
    where
        P: ImagePool,
    {
        Ok(Self {
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
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct ListAnnouncementInfosParams {
    /// Parent team whose announcements to list.
    pub team_id: String,

    /// Related rows to embed. Repeatable. Values: `user`.
    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub incl_opt: Vec<AnnouncementInclOpt>,

    pub offset: u32,
    pub limit: u32,
}

impl From<ListAnnouncementInfosParams> for AnnouncementListSpec {
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
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateAnnouncementParams {
    pub team_id: String,

    pub title: String,
    pub content: String,
}

/// Return value from creating an announcement.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateAnnouncementPayload {
    pub id: String,
}
