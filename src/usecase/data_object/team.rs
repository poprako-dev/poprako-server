use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::domain::external::image_pool::ImageGet;
use crate::domain::model::aggr::team::TeamAggr;

/// Public-facing representation of a translation team.
#[derive(Debug, Serialize, ToSchema)]
pub struct TeamBase {
    pub id: String,

    pub name: String,
    pub description: String,

    pub avatar_url: Option<String>,

    pub workset_next_index: i32,

    pub created_at: i64,
    pub updated_at: i64,
}

impl TeamBase {
    pub async fn from_aggr<S>(aggr: TeamAggr, signer: &S) -> Self
    where
        S: ImageGet,
    {
        let avatar_url = if aggr.avatar_uploaded {
            signer
                .get_signed(&aggr.avatar_key)
                .await
                .ok()
                .map(|url| url.to_string())
        } else {
            None
        };

        Self {
            id: aggr.id,
            name: aggr.name,
            description: aggr.description,
            avatar_url,
            workset_next_index: aggr.workset_next_index,
            created_at: aggr.created_at.to_unix_milli(),
            updated_at: aggr.updated_at.to_unix_milli(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TeamCreateParams {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TeamUpdateParams {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReserveTeamAvatarParams {
    pub file_extension: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReserveTeamAvatarReply {
    pub put_url: String,
}
