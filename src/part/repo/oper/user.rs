use poprako_orchestra::Oper;

use crate::model::user::{
    UserAvatarReservation, UserCredential, UserEntry, UserInfo,
};

pub struct CreateUser<'a> {
    pub entry: &'a UserEntry,
}

impl<'a> Oper for CreateUser<'a> {
    type Output = UserInfo;
}

pub enum GetUserInfo<'a> {
    Id { id: &'a str },
}

impl<'a> Oper for GetUserInfo<'a> {
    type Output = UserInfo;
}

pub enum GetUserCredential<'a> {
    Qid { qid: &'a str },
}

impl<'a> Oper for GetUserCredential<'a> {
    type Output = UserCredential;
}

pub enum FindUserInfo<'a> {
    Qid { qid: &'a str },
}

impl<'a> Oper for FindUserInfo<'a> {
    type Output = Option<UserInfo>;
}

pub enum UpdateUser<'a> {
    Info {
        id: &'a str,
        qid: &'a str,
        nickname: &'a str,
    },
    MarkAvatarUploaded {
        id: &'a str,
        avatar_version: u32,
    },
    TouchLastActive {
        id: &'a str,
    },
    PasswordHash {
        id: &'a str,
        password_hash: &'a str,
    },
}

impl<'a> Oper for UpdateUser<'a> {
    type Output = ();
}

pub struct ReserveUserAvatar<'a> {
    pub id: &'a str,
    pub file_ext: &'a str,
}

impl<'a> Oper for ReserveUserAvatar<'a> {
    type Output = UserAvatarReservation;
}

pub enum GetUserInfoExcluded<'a> {
    Id { id: &'a str },
}

impl<'a> Oper for GetUserInfoExcluded<'a> {
    type Output = UserInfo;
}

pub struct DeleteUser<'a> {
    pub id: &'a str,
}

impl<'a> Oper for DeleteUser<'a> {
    type Output = ();
}
