use poprako_orchestra::Oper;

use crate::model::user::{
    UserAvatarReservation, UserCredential, UserEntry, UserInfo,
};
use crate::value::image::{ImageExt, ImageHash};

pub struct CreateUser<'a> {
    pub entry: &'a UserEntry,
}

impl Oper for CreateUser<'_> {
    type Output = UserInfo;
}

pub enum GetUserInfo<'a> {
    Id { id: &'a str },
}

impl Oper for GetUserInfo<'_> {
    type Output = UserInfo;
}

pub enum GetUserCredential<'a> {
    Qid { qid: &'a str },
}

impl Oper for GetUserCredential<'_> {
    type Output = UserCredential;
}

pub enum FindUserInfo<'a> {
    Qid { qid: &'a str },
}

impl Oper for FindUserInfo<'_> {
    type Output = Option<UserInfo>;
}

pub enum UpdateUser<'a> {
    Info {
        //
        id: &'a str,
        qid: &'a str,
        nickname: &'a str,
    },

    MarkAvatarUploaded {
        //
        id: &'a str,
        avatar_version: u32,
        avatar_key: Option<&'a str>,
        avatar_uploaded: bool,
    },

    TouchLastActive {
        id: &'a str,
    },

    PasswordHash {
        //
        id: &'a str,
        password_hash: &'a str,
    },
}

impl Oper for UpdateUser<'_> {
    type Output = ();
}

pub struct ReserveUserAvatar<'a> {
    //
    pub id: &'a str,
    pub image_hash: &'a ImageHash,
    pub image_ext: ImageExt,
}

impl Oper for ReserveUserAvatar<'_> {
    type Output = UserAvatarReservation;
}

pub enum GetUserInfoExcluded<'a> {
    Id { id: &'a str },
}

impl Oper for GetUserInfoExcluded<'_> {
    type Output = UserInfo;
}

pub struct DeleteUser<'a> {
    pub id: &'a str,
}

impl Oper for DeleteUser<'_> {
    type Output = ();
}
