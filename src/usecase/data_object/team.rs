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
        let avatar_url = match (aggr.avatar_uploaded, &aggr.avatar_key) {
            (true, Some(key)) => signer.get_signed(key).await.ok(),
            _ => None,
        };

        Self {
            id: aggr.id,
            name: aggr.name,
            description: aggr.description,
            avatar_url: avatar_url.map(Into::into),
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
pub struct TeamInfoUpdateParams {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TeamAvatarReserveParams {
    pub file_extension: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TeamAvatarReserveReply {
    pub put_url: String,
    pub image_version: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TeamAvatarMarkUploadedParams {
    pub image_version: i64,
}
