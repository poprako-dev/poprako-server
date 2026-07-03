//! Data transfer objects for comment use cases.

use serde::{Deserialize, Serialize};

use utoipa::{IntoParams, ToSchema};

use poprako_macro::Paginate;
use poprako_util::time::ToUnixMilli;

use crate::data::user::UserInfoVal;
use crate::model::comment::{CommentInfo, CommentListSpec};
use crate::part::image::ImagePool;
use crate::result::RegularResult;
use crate::value::comment::CommentInclOpt;

/// Presentation-ready team board comment information.
#[derive(Debug, Serialize, ToSchema)]
pub struct CommentInfoVal {
    pub id: String,

    pub team_id: String,
    pub user_id: String,
    pub user: Option<UserInfoVal>,

    pub content: String,

    pub created_at: i64,
}

impl CommentInfoVal {
    /// Converts a comment model into a presentation value.
    pub async fn from_model<P>(image_pool: &P, model: CommentInfo) -> RegularResult<Self>
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
            content: model.content,
            created_at: model.created_at.to_unix_milli(),
        })
    }
}

/// Input parameters for listing comments.
#[Paginate]
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListCommentInfosData {
    pub team_id: String,

    #[serde(default, rename = "incl")]
    pub incl_opt: Vec<CommentInclOpt>,
}

impl From<ListCommentInfosData> for CommentListSpec {
    fn from(data: ListCommentInfosData) -> Self {
        Self {
            team_id: data.team_id,
            incl_opt: data.incl_opt,
            offset: data.offset,
            limit: data.limit,
        }
    }
}

/// Input parameters for creating a comment.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCommentData {
    pub team_id: String,
    pub content: String,
}

/// Return value from creating a comment.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateCommentVal {
    pub id: String,
}
