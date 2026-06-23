use poprako_util::time::ToUnixMilli;

use crate::model::team::TeamInfo as TeamInfoModel;
use crate::part::image::ImagePool;
use crate::result::RootResult;

pub struct TeamInfoVal {
    pub id: String,
    pub name: String,
    pub description: String,
    pub avatar_url: Option<String>,
    pub workset_next_index: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

impl TeamInfoVal {
    pub async fn from_model<P>(image_pool: &P, model: TeamInfoModel) -> RootResult<Self>
    where
        P: ImagePool,
    {
        let avatar_url = match (model.avatar_uploaded, &model.avatar_key) {
            (true, Some(key)) => image_pool.get_signed(key).await.ok(),
            _ => None,
        };

        Ok(Self {
            id: model.id,
            name: model.name,
            description: model.description,
            avatar_url: avatar_url.map(Into::into),
            workset_next_index: model.workset_next_index,
            created_at: model.created_at.to_unix_milli(),
            updated_at: model.updated_at.to_unix_milli(),
        })
    }
}

pub struct CreateTeamData {
    pub name: String,
    pub description: String,
}

pub struct UpdateTeamInfoData {
    pub id: String,
    pub name: String,
    pub description: String,
}

pub struct ReserveTeamAvatarData {
    pub file_ext: String,
}

pub struct ReserveTeamAvatarVal {
    pub put_url: String,
    pub avatar_version: i64,
}

pub struct MarkTeamAvatarUploadedData {
    pub avatar_version: i64,
}
