//! Data transfer objects for team profile use cases.

use poprako_macro::Paginate;
use poprako_util::time::ToUnixMilli;

use crate::model::team::TeamInfo;
use crate::part::image::ImagePool;
use crate::result::RegularResult;

/// Presentation-ready team profile information.
///
/// Converts the raw [`TeamInfoModel`] timestamps to Unix milliseconds and
/// resolves the avatar key to a signed download URL via [`ImagePool`] when
/// the avatar has been uploaded.
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
    /// Converts a [`TeamInfoModel`] into a presentation-ready value.
    ///
    /// Resolves a signed avatar download URL when the avatar has
    /// been uploaded and a key is present. Timestamps are converted
    /// from [`OffsetDateTime`] to Unix milliseconds.
    ///
    /// [`OffsetDateTime`]: time::OffsetDateTime
    pub async fn from_model<P>(image_pool: &P, model: TeamInfo) -> RegularResult<Self>
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

/// Input parameters for creating a new team.
pub struct CreateTeamData {
    pub name: String,
    pub description: String,
}

/// Input parameters for listing teams.
///
/// When `user_id` is [`None`], the request lists all teams and must be made
/// by a super-admin. When `user_id` is [`Some`], the request lists teams joined
/// by that user.
#[Paginate]
pub struct ListTeamInfosData {
    pub user_id: Option<String>,
}

/// Input parameters for updating a team's profile.
pub struct UpdateTeamInfoData {
    pub id: String,

    pub name: String,
    pub description: String,
}

/// Input parameters for reserving a new team avatar upload slot.
pub struct ReserveTeamAvatarData {
    pub file_ext: String,
}

/// Return value from a successful team avatar reservation.
pub struct ReserveTeamAvatarVal {
    pub put_url: String,
    pub avatar_version: i64,
}

/// Input parameters for confirming a team avatar upload completed.
pub struct MarkTeamAvatarUploadedData {
    pub avatar_version: i64,
}
