use crate::model::user::UserInfo;
use crate::part::image_pool::ImagePool;

pub struct UserInfoVal {
    pub id: String,
}

impl UserInfoVal {
    pub async fn from_model<P>(image_pool: &P, model: UserInfo) -> Self
    where
        P: ImagePool,
    {
        todo!()
    }
}

pub struct UserInfoUpdData {
    pub id: String,

    pub qid: String,
    pub nickname: String,
}
