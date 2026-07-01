//! Data transfer objects for announcement use cases.

use serde::Deserialize;

use poprako_macro::Paginate;
use poprako_util::time::ToUnixMilli;

use crate::data::user::UserInfoVal;
use crate::model::announcement::{AnnouncementInfo, AnnouncementListSpec};
use crate::part::image::ImagePool;
use crate::result::RegularResult;
use crate::value::announcement::AnnouncementInclOpt;

/// Presentation-ready team announcement information.
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
    pub async fn from_model<P>(image_pool: &P, model: AnnouncementInfo) -> RegularResult<Self>
    where
        P: ImagePool,
    {
        let user = match model.user {
            Some(user_info) => Some(UserInfoVal::from_model(image_pool, user_info).await?),
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
#[Paginate]
#[derive(Deserialize)]
pub struct ListAnnouncementInfosData {
    pub team_id: String,

    #[serde(default)]
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
pub struct CreateAnnouncementData {
    pub team_id: String,

    pub title: String,
    pub content: String,
}

/// Return value from creating an announcement.
pub struct CreateAnnouncementVal {
    pub id: String,
}
