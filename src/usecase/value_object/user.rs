use crate::domain::external::oss::OssGetSigner;
use crate::domain::model::aggregate::user::UserAggr;
use crate::util::time::ToUnixMilli as _;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserVal {
    pub id: String,

    pub nickname: String,
    pub qid: String,

    pub avatar_url: Option<String>,

    pub is_sadmin: bool,

    pub last_active_at: i64,

    pub created_at: i64,
    pub updated_at: i64,
}

impl UserVal {
    pub fn from_aggr<S>(aggr: UserAggr, signer: S) -> Self
    where
        S: OssGetSigner,
    {
        Self {
            id: aggr.id,
            qid: aggr.qid,
            nickname: aggr.nickname,
            avatar_url: aggr
                .avatar_uploaded
                .then(|| signer.sign_get(&aggr.avatar_key))
                .flatten()
                .map(|url| url.to_string()),
            is_sadmin: aggr.is_sadmin,
            last_active_at: aggr.last_active_at.to_unix_milli(),
            created_at: aggr.created_at.to_unix_milli(),
            updated_at: aggr.updated_at.to_unix_milli(),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RegisterUserParams {
    pub qid: String,
    pub nickname: String,
    pub password: String,
    pub invitation_code: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RegisterUserReply {
    pub user_id: String,
    pub token: String,
}
