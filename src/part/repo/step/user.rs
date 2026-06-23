//! Step types for user repository operations.

use poprako_transactional::step::Step;

use crate::model::user::{UserAvatarReservation, UserCredential, UserForm, UserInfo};

/// Step that fetches a user by their identifier.
pub struct GetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoById<'a> {
    type Output = UserInfo;
}

/// Step that fetches a user's stored credential by QQ ID.
pub struct GetCredentialByQid<'a> {
    pub qid: &'a str,
}

impl<'a> Step for GetCredentialByQid<'a> {
    type Output = UserCredential;
}

/// Step that inserts a new user row.
pub struct Create<'a> {
    pub form: &'a UserForm,
}

impl<'a> Step for Create<'a> {
    type Output = UserInfo;
}

/// Step that updates a user's QQ ID and nickname.
pub struct UpdateInfo<'a> {
    pub id: &'a str,
    pub qid: &'a str,
    pub nickname: &'a str,
}

impl<'a> Step for UpdateInfo<'a> {
    type Output = ();
}

/// Step that reserves a new avatar upload slot for a user.
///
/// Generates an object key, increments the avatar version, and records
/// the previous avatar key (if any) for later cleanup.
pub struct ReserveAvatar<'a> {
    pub id: &'a str,
    pub file_ext: &'a str,
}

impl<'a> Step for ReserveAvatar<'a> {
    type Output = UserAvatarReservation;
}

/// Step that marks a reserved avatar as successfully uploaded.
///
/// `avatar_version` must match the version returned by [`ReserveAvatar`].
pub struct MarkAvatarUploaded<'a> {
    pub id: &'a str,
    pub avatar_version: i64,
}

impl<'a> Step for MarkAvatarUploaded<'a> {
    type Output = ();
}

/// Step that updates a user's `last_active_at` timestamp.
pub struct TouchLastActive<'a> {
    pub id: &'a str,
}

impl<'a> Step for TouchLastActive<'a> {
    type Output = ();
}

/// Step that fetches a user by ID with a pessimistic lock.
///
/// The `Excluded` suffix indicates this query uses `FOR UPDATE` (or
/// equivalent) to prevent concurrent modification during a transaction.
pub struct GetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoExcluded<'a> {
    type Output = UserInfo;
}

/// Step that deletes a user by their identifier.
pub struct Delete<'a> {
    pub id: &'a str,
}

impl<'a> Step for Delete<'a> {
    type Output = ();
}

/// Factory for constructing user repository [`Step`] values.
pub struct UserStep;

impl UserStep {
    /// Constructs a step to fetch a user by ID.
    pub fn get_info_by_id<'a>(id: &'a str) -> GetInfoById<'a> {
        GetInfoById { id }
    }

    /// Constructs a step to fetch a user's credential by QQ ID.
    pub fn get_credential_by_qid<'a>(qid: &'a str) -> GetCredentialByQid<'a> {
        GetCredentialByQid { qid }
    }

    /// Constructs a step to insert a new user.
    pub fn create<'a>(form: &'a UserForm) -> Create<'a> {
        Create { form }
    }

    /// Constructs a step to update a user's QQ ID and nickname.
    pub fn update_info<'a>(id: &'a str, qid: &'a str, nickname: &'a str) -> UpdateInfo<'a> {
        UpdateInfo { id, qid, nickname }
    }

    /// Constructs a step to reserve a new avatar upload slot.
    pub fn reserve_avatar<'a>(id: &'a str, file_ext: &'a str) -> ReserveAvatar<'a> {
        ReserveAvatar { id, file_ext }
    }

    /// Constructs a step to confirm an avatar upload completed.
    pub fn mark_avatar_uploaded<'a>(id: &'a str, avatar_version: i64) -> MarkAvatarUploaded<'a> {
        MarkAvatarUploaded { id, avatar_version }
    }

    /// Constructs a step to update the last-active timestamp.
    pub fn touch_last_active<'a>(id: &'a str) -> TouchLastActive<'a> {
        TouchLastActive { id }
    }

    /// Constructs a step to fetch a user with a pessimistic lock.
    pub fn get_info_excluded<'a>(id: &'a str) -> GetInfoExcluded<'a> {
        GetInfoExcluded { id }
    }

    /// Constructs a step to delete a user.
    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }
}
