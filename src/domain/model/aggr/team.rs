use time::OffsetDateTime;
use uuid::Uuid;

/// Read-model aggregate for a translation team.
#[cfg_attr(test, derive(Clone))]
pub struct TeamAggr {
    pub id: String,

    pub name: String,
    pub description: String,

    pub avatar_key: String,
    pub avatar_uploaded: bool,
    pub avatar_version: i64,

    pub workset_next_index: i32,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl TeamAggr {
    pub fn generate_id() -> String {
        format!("team-{}", Uuid::now_v7())
    }

    /// Returns the OSS object key for the team avatar with the given file extension.
    pub fn generate_avatar_key(team_id: &str, image_version: i64, ext: &str) -> String {
        format!("team_avatar/{}-{}.{}", team_id, image_version, ext)
    }
}

pub struct TeamAvatarReservation {
    pub object_key: String,
    pub previous_object_key: Option<String>,
    pub image_version: i64,
}

/// Input aggregate for creating a new team.
///
/// The caller must generate `id` via [`TeamAggr::generate_id`].
pub struct TeamForm {
    pub id: String,

    pub name: String,
    pub description: String,
}

/// Input aggregate for updating a team (PUT semantics).
///
/// The caller provides the existing team `id`.
pub struct TeamInfoUpdate {
    pub id: String,

    pub name: String,
    pub description: String,
}
