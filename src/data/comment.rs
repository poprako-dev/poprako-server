//! Data transfer objects for comment use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::{IntoParams, ToSchema};

use poprako_util::time::ToUnixMilli;

use crate::data::user_data;
use crate::model::comment_model;
use crate::part::image::ImagePool;
use crate::result::RegularResult;
use crate::value::comment::CommentInclOpt;

/// Presentation-ready team board comment information.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct InfoVal {
    pub id: String,

    pub team_id: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<user_data::InfoVal>,

    pub content: String,

    pub created_at: i64,
}

impl InfoVal {
    /// Converts a comment model into a presentation value.
    pub async fn from_model<P>(
        image_pool: &P,
        model: comment_model::Info,
    ) -> RegularResult<Self>
    where
        P: ImagePool,
    {
        let user = match model.user {
            //
            Some(user_info) => Some(
                user_data::InfoVal::from_model(image_pool, user_info).await?,
            ),

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
///
/// `incl` embeds related rows into each item.
///
/// Example: `/api/v1/teams/{team_id}/comments?incl=user&offset=0&limit=20`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct ListInfosData {
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

impl From<ListInfosData> for comment_model::ListSpec {
    fn from(data: ListInfosData) -> Self {
        Self {
            team_id: data.team_id,
            incl_opt: data.incl_opt,
            offset: data.offset,
            limit: data.limit,
        }
    }
}

/// Input parameters for creating a comment.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateData {
    pub team_id: String,
    pub content: String,
}

/// Return value from creating a comment.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct CreateVal {
    pub id: String,
}
