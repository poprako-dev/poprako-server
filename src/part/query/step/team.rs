use poprako_transactional::step::Step;
use poprako_util::page::Page;

use crate::model::team::{
    TeamAvatarReservation,
    TeamForm,
    TeamInfo,
    TeamInfoUpdate,
};

pub struct TeamCreate<'a> {
    pub form: &'a TeamForm,
}

impl<'a> Step for TeamCreate<'a> {
    type Output = TeamInfo;
}

pub struct TeamGetInfoById<'a> {
    pub id: &'a str,
}

impl<'a> Step for TeamGetInfoById<'a> {
    type Output = TeamInfo;
}

pub struct TeamList {
    pub page: Page,
}

impl Step for TeamList {
    type Output = Vec<TeamInfo>;
}

pub struct TeamUpdateInfo<'a> {
    pub input: TeamInfoUpdate<'a>,
}

impl<'a> Step for TeamUpdateInfo<'a> {
    type Output = ();
}

pub struct TeamReserveAvatar<'a> {
    pub id: &'a str,
    pub file_extension: &'a str,
}

impl<'a> Step for TeamReserveAvatar<'a> {
    type Output = TeamAvatarReservation;
}

pub struct TeamMarkAvatarUploaded<'a> {
    pub id: &'a str,
    pub avatar_version: i64,
}

impl<'a> Step for TeamMarkAvatarUploaded<'a> {
    type Output = ();
}

pub struct TeamGetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Step for TeamGetInfoExcluded<'a> {
    type Output = TeamInfo;
}

pub struct TeamDelete<'a> {
    pub id: &'a str,
}

impl<'a> Step for TeamDelete<'a> {
    type Output = ();
}
