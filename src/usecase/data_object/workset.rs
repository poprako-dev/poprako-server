use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use poprako_util::time::ToUnixMilli as _;

use crate::domain::external::image_pool::ImageGet;
use crate::domain::model::aggr::workset::WorksetAggr;
use crate::usecase::data_object::team::TeamInfo;

/// Public-facing representation of a workset.
#[derive(Debug, Serialize, ToSchema)]
pub struct WorksetInfo {
    pub id: String,

    pub team_id: String,
    pub team: Option<TeamInfo>,

    pub index: i32,
    pub name: String,
    pub description: Option<String>,
    pub comic_count: i32,
    pub comic_next_index: i32,

    pub created_at: i64,
    pub updated_at: i64,
}

impl WorksetInfo {
    pub async fn from_aggr<S>(aggr: WorksetAggr, signer: &S) -> Self
    where
        S: ImageGet,
    {
        let team = if let Some(t) = aggr.team {
            Some(TeamInfo::from_aggr(t, signer).await)
        } else {
            None
        };

        Self {
            id: aggr.id,
            team_id: aggr.team_id,
            team,
            index: aggr.index,
            name: aggr.name,
            description: aggr.description,
            comic_count: aggr.comic_count,
            comic_next_index: aggr.comic_next_index,
            created_at: aggr.created_at.to_unix_milli(),
            updated_at: aggr.updated_at.to_unix_milli(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WorksetCreateParams {
    pub team_id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct WorksetUpdateParams {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WorksetCreateReply {
    pub id: String,
}
