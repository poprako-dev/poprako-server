//! Domain models for team profile storage.

use time::OffsetDateTime;

/// A team（汉化组）record as stored in the database.
///
/// Carries raw [`OffsetDateTime`] timestamps; convert to [`TeamInfoVal`] for
/// presentation. The `workset_next_index` field is a monotonically increasing
/// counter used to assign indices to new worksets within this team.
///
/// [`TeamInfoVal`]: crate::data::team::TeamInfoVal
#[cfg_attr(test, derive(Clone))]
pub struct TeamInfo {
    pub id: String,

    pub name: String,
    pub description: String,

    pub avatar_key: Option<String>,
    pub avatar_uploaded: bool,
    pub avatar_version: i64,

    pub workset_next_index: i32,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// The data needed to create a new team.
#[cfg_attr(test, derive(Clone))]
pub struct TeamForm {
    pub id: String,

    pub name: String,
    pub description: String,
}

/// The result of reserving a new team avatar upload slot.
///
/// Mirrors [`UserAvatarReservation`] for the team domain. Contains the
/// generated object key, any previous key to clean up, and the version
/// that must match when the upload is confirmed.
///
/// [`UserAvatarReservation`]: crate::model::user::UserAvatarReservation
#[cfg_attr(test, derive(Clone))]
pub struct TeamAvatarReservation {
    pub object_key: String,
    pub prev_object_key: Option<String>,
    pub avatar_version: i64,
}
