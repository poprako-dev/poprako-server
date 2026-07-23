//! Data transfer objects for comment use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use futures::future::OptionFuture;

use poprako_util::time::ToUnixMilli;

use crate::data::user::UserInfoVal;
use crate::model::comment::{CommentInfo, CommentListSpec};
use crate::part::image::ImagePool;
use crate::result::{BaseResult, accept};
use crate::value::comment::CommentInclOpt;

/// Presentation-ready team board comment information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CommentInfoVal {
    pub id: String,

    pub team_id: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfoVal>,

    pub content: String,

    pub created_at: i64,
}

impl CommentInfoVal {
    /// Converts a comment model into a presentation value.
    pub async fn from_model<P>(
        image_pool: &P,
        model: CommentInfo,
    ) -> BaseResult<Self>
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
            content: model.content,
            created_at: model.created_at.to_unix_milli(),
        })
    }
}

/// Input parameters for listing comments.
///
/// `incl` embeds related rows into each item.
///
/// Example: `/api/v1/teams/{team_id}/comments?incl=user&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListCommentInfosParams {
    /// Parent team whose comments to list.
    pub team_id: String,

    /// Related rows to embed. Repeatable. Values: `user`.
    #[serde(
        default,
        rename = "incl",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub incl_opt: Vec<CommentInclOpt>,

    pub offset: u32,
    pub limit: u32,
}

impl From<ListCommentInfosParams> for CommentListSpec {
    fn from(params: ListCommentInfosParams) -> Self {
        Self {
            team_id: params.team_id,
            incl_opt: params.incl_opt,
            offset: params.offset,
            limit: params.limit,
        }
    }
}

/// Input parameters for creating a comment.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateCommentParams {
    pub team_id: String,
    pub content: String,
}

/// Return value from creating a comment.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateCommentPayload {
    pub id: String,
}
