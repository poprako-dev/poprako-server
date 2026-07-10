//! Data transfer objects for announcement use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::{IntoParams, ToSchema};

use poprako_macro::Paginate;
use poprako_util::time::ToUnixMilli;

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
        let user = match model.user {
            //
            Some(user_info) => {
                Some(UserInfoVal::from_model(image_pool, user_info).await?)
            }

            None => None,
        };

        Ok(Self {
            id: model.id,
            team_id: model.team_id,
            user_id: model.user_id,
            user,
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
#[Paginate]
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct ListAnnouncementInfosData {
    /// Parent team whose announcements to list.
    pub team_id: String,

    /// Related rows to embed. Repeatable. Values: `user`.
    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub incl_opt: Vec<AnnouncementInclOpt>,
}

impl From<ListAnnouncementInfosData> for AnnouncementListSpec {
    fn from(data: ListAnnouncementInfosData) -> Self {
        Self {
            team_id: data.team_id,
            incl_opt: data.incl_opt,
            offset: data.offset,
            limit: data.limit,
        }
    }
}

/// Input parameters for creating an announcement.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateAnnouncementData {
    pub team_id: String,

    pub title: String,
    pub content: String,
}

/// Return value from creating an announcement.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateAnnouncementVal {
    pub id: String,
}
