use poprako_transactional::step::Step;

use crate::model::user::{
    UserAvatarReservation, UserCredential, UserForm, UserInfo, UserInfoUpdate,
};

pub struct UserGetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for UserGetInfoById<'a> {
    type Output = UserInfo;
}

pub struct UserGetCredentialByQid<'a> {
    pub qid: &'a str,
}

impl<'a> Step for UserGetCredentialByQid<'a> {
    type Output = UserCredential;
}

pub struct UserCreate<'a> {
    pub form: &'a UserForm,
}

impl<'a> Step for UserCreate<'a> {
    type Output = UserInfo;
}

pub struct UserUpdateInfo<'a> {
    pub input: UserInfoUpdate<'a>,
}

impl<'a> Step for UserUpdateInfo<'a> {
    type Output = ();
}

pub struct UserReserveAvatar<'a> {
    pub id: &'a str,
    pub file_ext: &'a str,
}

impl<'a> Step for UserReserveAvatar<'a> {
    type Output = UserAvatarReservation;
}

pub struct UserMarkAvatarUploaded<'a> {
    pub id: &'a str,
    pub avatar_version: i64,
}

impl<'a> Step for UserMarkAvatarUploaded<'a> {
    type Output = ();
}

pub struct UserTouchLastActive<'a> {
    pub id: &'a str,
}

impl<'a> Step for UserTouchLastActive<'a> {
    type Output = ();
}

pub struct UserGetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for UserGetInfoExcluded<'a> {
    type Output = UserInfo;
}

pub struct UserDelete<'a> {
    pub id: &'a str,
}

impl<'a> Step for UserDelete<'a> {
    type Output = ();
}

pub struct UserStep;

impl UserStep {
    pub fn get_info_by_id<'a>(id: &'a str) -> UserGetInfoById<'a> {
        UserGetInfoById { id }
    }
}
