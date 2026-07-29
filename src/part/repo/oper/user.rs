use poprako_orchestra::Oper;

use crate::model::user::{
    UserAvatarReservation, UserCredential, UserEntry, UserInfo,
};
use crate::value::image::{ImageExt, ImageHash};

/// Creates a user.
pub struct CreateUser<'a> {
    /// The user entry to insert.
    pub entry: &'a UserEntry,
}

impl Oper for CreateUser<'_> {
    // Operation output type.
    type Output = UserInfo;
}

/// Looks up a user by identifier.
pub enum GetUserInfo<'a> {
    /// Fetch by user id.
    Id {
        /// The unique user identifier.
        id: &'a str,
    },
}

impl Oper for GetUserInfo<'_> {
    // Operation output type.
    type Output = UserInfo;
}

/// Looks up user credentials by OAuth qid.
pub enum GetUserCredential<'a> {
    /// Fetch by qid.
    Qid {
        /// The OAuth qualified identifier.
        qid: &'a str,
    },
}

impl Oper for GetUserCredential<'_> {
    // Operation output type.
    type Output = UserCredential;
}

/// Finds a user by OAuth qid, returning `None` if not found.
pub enum FindUserInfo<'a> {
    /// Fetch by qid.
    Qid {
        /// The OAuth qualified identifier.
        qid: &'a str,
    },
}

impl Oper for FindUserInfo<'_> {
    // Operation output type.
    type Output = Option<UserInfo>;
}

/// Updates a user.
pub enum UpdateUser<'a> {
    /// Updates user metadata fields.
    Info {
        //
        /// The unique user identifier.
        id: &'a str,
        /// The OAuth qualified identifier.
        qid: &'a str,
        /// The display nickname.
        nickname: &'a str,
    },

    /// Marks a user avatar as uploaded.
    MarkAvatarUploaded {
        //
        /// The unique user identifier.
        id: &'a str,
        /// The new avatar version number.
        avatar_version: u32,
        /// The object storage key.
        avatar_key: Option<&'a str>,
        /// Whether the upload has completed.
        avatar_uploaded: bool,
    },

    /// Touches the last-active timestamp.
    TouchLastActive {
        /// The unique user identifier.
        id: &'a str,
    },

    /// Updates the password hash.
    PasswordHash {
        //
        /// The unique user identifier.
        id: &'a str,
        /// The hashed password value.
        password_hash: &'a str,
    },
}

impl Oper for UpdateUser<'_> {
    // Operation output type.
    type Output = ();
}

/// Reserves a user avatar slot for an upload.
pub struct ReserveUserAvatar<'a> {
    //
    /// The user id.
    pub id: &'a str,

    /// The image hash for deduplication.
    pub image_hash: &'a ImageHash,

    /// The image file extension.
    pub image_ext: ImageExt,
}

impl Oper for ReserveUserAvatar<'_> {
    // Operation output type.
    type Output = UserAvatarReservation;
}

/// Looks up a user by identifier, matching deleted rows as well.
pub enum GetUserInfoExcluded<'a> {
    /// Fetch by user id.
    Id {
        /// The unique user identifier.
        id: &'a str,
    },
}

impl Oper for GetUserInfoExcluded<'_> {
    // Operation output type.
    type Output = UserInfo;
}

/// Deletes a user.
pub struct DeleteUser<'a> {
    /// The user id.
    pub id: &'a str,
}

impl Oper for DeleteUser<'_> {
    // Operation output type.
    type Output = ();
}
