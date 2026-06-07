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

    pub workset_next_index: i32,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl TeamAggr {
    pub fn generate_id() -> String {
        format!("team-{}", Uuid::now_v7())
    }

    /// Returns the OSS object key for the team avatar with the given file extension.
    pub fn generate_avatar_key(&self, ext: &str) -> String {
        format!("team_avatar/{}.{}", self.id, ext)
    }
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
pub struct TeamUpdate {
    pub id: String,

    pub name: String,
    pub description: String,
}
