use poprako_transactional::step::Step;

use crate::model::user::{UserAvatarReservation, UserCredential, UserForm, UserInfo};

pub struct GetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoById<'a> {
    type Output = UserInfo;
}

pub struct GetCredentialByQid<'a> {
    pub qid: &'a str,
}

impl<'a> Step for GetCredentialByQid<'a> {
    type Output = UserCredential;
}

pub struct Create<'a> {
    pub form: &'a UserForm,
}

impl<'a> Step for Create<'a> {
    type Output = UserInfo;
}

pub struct UpdateInfo<'a> {
    pub id: &'a str,
    pub qid: &'a str,
    pub nickname: &'a str,
}

impl<'a> Step for UpdateInfo<'a> {
    type Output = ();
}

pub struct ReserveAvatar<'a> {
    pub id: &'a str,
    pub file_ext: &'a str,
}

impl<'a> Step for ReserveAvatar<'a> {
    type Output = UserAvatarReservation;
}

pub struct MarkAvatarUploaded<'a> {
    pub id: &'a str,
    pub avatar_version: i64,
}

impl<'a> Step for MarkAvatarUploaded<'a> {
    type Output = ();
}

pub struct TouchLastActive<'a> {
    pub id: &'a str,
}

impl<'a> Step for TouchLastActive<'a> {
    type Output = ();
}

pub struct GetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoExcluded<'a> {
    type Output = UserInfo;
}

pub struct Delete<'a> {
    pub id: &'a str,
}

impl<'a> Step for Delete<'a> {
    type Output = ();
}

pub struct UserStep;

impl UserStep {
    pub fn get_info_by_id<'a>(id: &'a str) -> GetInfoById<'a> {
        GetInfoById { id }
    }

    pub fn get_credential_by_qid<'a>(qid: &'a str) -> GetCredentialByQid<'a> {
        GetCredentialByQid { qid }
    }

    pub fn create<'a>(form: &'a UserForm) -> Create<'a> {
        Create { form }
    }

    pub fn update_info<'a>(id: &'a str, qid: &'a str, nickname: &'a str) -> UpdateInfo<'a> {
        UpdateInfo { id, qid, nickname }
    }

    pub fn reserve_avatar<'a>(id: &'a str, file_ext: &'a str) -> ReserveAvatar<'a> {
        ReserveAvatar { id, file_ext }
    }

    pub fn mark_avatar_uploaded<'a>(id: &'a str, avatar_version: i64) -> MarkAvatarUploaded<'a> {
        MarkAvatarUploaded { id, avatar_version }
    }

    pub fn touch_last_active<'a>(id: &'a str) -> TouchLastActive<'a> {
        TouchLastActive { id }
    }

    pub fn get_info_excluded<'a>(id: &'a str) -> GetInfoExcluded<'a> {
        GetInfoExcluded { id }
    }

    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }
}
