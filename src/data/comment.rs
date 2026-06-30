//! Data transfer objects for comment use cases.

use serde::Deserialize;

use poprako_macro::Paginate;
use poprako_util::time::ToUnixMilli;

use crate::data::user::UserInfoVal;
use crate::model::comment::{CommentInfo, CommentListSpec};
use crate::part::image::ImagePool;
use crate::result::RootResult;
use crate::value::comment::CommentInclOpt;

/// Presentation-ready team board comment information.
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
    pub async fn from_model<P>(image_pool: &P, model: CommentInfo) -> RootResult<Self>
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
#[derive(Deserialize)]
pub struct ListCommentInfosData {
    pub team_id: String,

    #[serde(default)]
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
pub struct CreateCommentData {
    pub team_id: String,
    pub content: String,
}

/// Return value from creating a comment.
pub struct CreateCommentVal {
    pub id: String,
}
