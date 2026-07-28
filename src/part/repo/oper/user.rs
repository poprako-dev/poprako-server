use poprako_orchestra::Oper;

use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::write::user::{
    UserAvatarRepl, UserAvatarReservation, UserCredsRepl, UserEntry,
    UserInfoRepl,
};
use crate::value::image::{ImageExt, ImageHash};

/// Creates a user.
#[derive(Oper)]
#[oper(output = UserInfo)]
pub struct CreateUser<'a> {
    /// The user entry to insert.
    pub entry: &'a UserEntry,
}

/// Looks up a user by identifier.
#[derive(Oper)]
#[oper(output = UserInfo)]
pub enum GetUserInfo<'a> {
    /// Fetch by user id.
    Id {
        /// The unique user identifier.
        id: &'a str,
    },
}

/// Looks up user credentials by OAuth qid.
#[derive(Oper)]
#[oper(output = UserCredential)]
pub enum GetUserCredential<'a> {
    /// Fetch by qid.
    Qid {
        /// The OAuth qualified identifier.
        qid: &'a str,
    },
}

/// Finds a user by OAuth qid, returning `None` if not found.
#[derive(Oper)]
#[oper(output = Option<UserInfo>)]
pub enum FindUserInfo<'a> {
    /// Fetch by qid.
    Qid {
        /// The OAuth qualified identifier.
        qid: &'a str,
    },
}

/// Updates a user.
#[derive(Oper)]
#[oper(output = ())]
pub enum UpdateUser<'a> {
    //
    /// Updates user metadata fields.
    Info {
        /// The replacement payload.
        repl: &'a UserInfoRepl,
    },

    /// Marks a user avatar as uploaded.
    MarkAvatarUploaded {
        /// The replacement payload.
        repl: &'a UserAvatarRepl,
    },

    /// Touches the last-active timestamp.
    TouchLastActive {
        /// The unique user identifier.
        id: &'a str,
    },

    /// Updates the password hash.
    PasswordHash {
        /// The replacement payload.
        repl: &'a UserCredsRepl,
    },
}

/// Reserves a user avatar slot for an upload.
#[derive(Oper)]
#[oper(output = UserAvatarReservation)]
pub struct ReserveUserAvatar<'a> {
    //
    /// The user id.
    pub id: &'a str,

    /// The image hash for deduplication.
    pub image_hash: &'a ImageHash,

    /// The image file extension.
    pub image_ext: ImageExt,
}

/// Looks up a user by identifier, matching deleted rows as well.
#[derive(Oper)]
#[oper(output = UserInfo)]
pub enum GetUserInfoExcluded<'a> {
    /// Fetch by user id.
    Id {
        /// The unique user identifier.
        id: &'a str,
    },
}

/// Deletes a user.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteUser<'a> {
    /// The user id.
    pub id: &'a str,
}
