use poprako_transactional::step::Step;
use poprako_util::page::Page;

use crate::model::team::{TeamAvatarReservation, TeamForm, TeamInfo};

pub struct Create<'a> {
    pub form: &'a TeamForm,
}

impl<'a> Step for Create<'a> {
    type Output = TeamInfo;
}

pub struct GetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoById<'a> {
    type Output = TeamInfo;
}

pub struct List {
    pub page: Page,
}

impl Step for List {
    type Output = Vec<TeamInfo>;
}

pub struct UpdateInfo<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: &'a str,
}

impl<'a> Step for UpdateInfo<'a> {
    type Output = ();
}

pub struct ReserveAvatar<'a> {
    pub id: &'a str,
    pub file_extension: &'a str,
}

impl<'a> Step for ReserveAvatar<'a> {
    type Output = TeamAvatarReservation;
}

pub struct MarkAvatarUploaded<'a> {
    pub id: &'a str,
    pub avatar_version: i64,
}

impl<'a> Step for MarkAvatarUploaded<'a> {
    type Output = ();
}

pub struct GetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for GetInfoExcluded<'a> {
    type Output = TeamInfo;
}

pub struct Delete<'a> {
    pub id: &'a str,
}

impl<'a> Step for Delete<'a> {
    type Output = ();
}

pub struct TeamStep;

impl TeamStep {
    pub fn create<'a>(form: &'a TeamForm) -> Create<'a> {
        Create { form }
    }

    pub fn get_info_by_id<'a>(id: &'a str) -> GetInfoById<'a> {
        GetInfoById { id }
    }

    pub fn list(page: Page) -> List {
        List { page }
    }

    pub fn update_info<'a>(id: &'a str, name: &'a str, description: &'a str) -> UpdateInfo<'a> {
        UpdateInfo {
            id,
            name,
            description,
        }
    }

    pub fn reserve_avatar<'a>(id: &'a str, file_extension: &'a str) -> ReserveAvatar<'a> {
        ReserveAvatar { id, file_extension }
    }

    pub fn mark_avatar_uploaded<'a>(id: &'a str, avatar_version: i64) -> MarkAvatarUploaded<'a> {
        MarkAvatarUploaded { id, avatar_version }
    }

    pub fn get_info_excluded<'a>(id: &'a str) -> GetInfoExcluded<'a> {
        GetInfoExcluded { id }
    }

    pub fn delete<'a>(id: &'a str) -> Delete<'a> {
        Delete { id }
    }
}
