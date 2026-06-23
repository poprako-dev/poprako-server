use poprako_util::time::ToUnixMilli;

use crate::model::user::UserInfo as UserInfoModel;
use crate::part::image::ImagePool;
use crate::result::RootResult;

pub struct UserInfoVal {
    pub id: String,

    pub nickname: String,
    pub qid: String,

    pub avatar_url: Option<String>,
    pub is_sadmin: bool,
    pub last_active_at: i64,

    pub created_at: i64,
    pub updated_at: i64,
}

impl UserInfoVal {
    pub async fn from_model<P>(image_pool: &P, model: UserInfoModel) -> RootResult<Self>
    where
        P: ImagePool,
    {
        let avatar_url = match (model.avatar_uploaded, &model.avatar_key) {
            (true, Some(key)) => image_pool.get_signed(key).await.ok(),
            _ => None,
        };

        Ok(Self {
            id: model.id,
            nickname: model.nickname,
            qid: model.qid,
            avatar_url: avatar_url.map(Into::into),
            is_sadmin: model.is_sadmin,
            last_active_at: model.last_active_at.to_unix_milli(),
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        })
    }
}

pub struct UpdateUserInfoData {
    pub id: String,

    pub qid: String,
    pub nickname: String,
}

pub struct ReserveUserAvatarData {
    pub file_ext: String,
}

pub struct ReserveUserAvatarVal {
    pub put_url: String,
    pub avatar_version: i64,
}

pub struct MarkUserAvatarUploadedData {
    pub avatar_version: i64,
}
