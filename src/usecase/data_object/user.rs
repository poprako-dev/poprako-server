use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::external::image_pool::ImageGet;
use crate::domain::model::aggr::user::UserAggr;
use poprako_util::time::ToUnixMilli as _;

#[derive(Debug, Deserialize, ToSchema)]
pub struct UserBase {
    pub id: String,

    pub nickname: String,
    pub qid: String,

    pub avatar_url: Option<String>,

    pub is_sadmin: bool,

    pub last_active_at: i64,

    pub created_at: i64,
    pub updated_at: i64,
}

impl UserBase {
    pub async fn from_aggr<S>(aggr: UserAggr, signer: &S) -> Self
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
            qid: aggr.qid,
            nickname: aggr.nickname,
            avatar_url,
            is_sadmin: aggr.is_sadmin,
            last_active_at: aggr.last_active_at.to_unix_milli(),
            created_at: aggr.created_at.to_unix_milli(),
            updated_at: aggr.updated_at.to_unix_milli(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SignUpUserParams {
    pub qid: String,
    pub nickname: String,
    pub password: String,
    pub invitation_code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SignUpUserReply {
    pub user_id: String,
    pub token: String,
}
