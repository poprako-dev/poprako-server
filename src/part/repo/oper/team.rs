use poprako_orchestra::Oper;

use crate::model::team::{
    TeamAvatarReservation, TeamEntry, TeamInfo, TeamInfoListSpec,
};

pub struct CreateTeam<'a> {
    pub entry: &'a TeamEntry,
}

impl<'a> Oper for CreateTeam<'a> {
    type Output = TeamInfo;
}

pub enum GetTeamInfo<'a> {
    Id { id: &'a str },
}

impl<'a> Oper for GetTeamInfo<'a> {
    type Output = TeamInfo;
}

pub struct ListTeamInfos<'a> {
    pub spec: &'a TeamInfoListSpec,
}

impl<'a> Oper for ListTeamInfos<'a> {
    type Output = Vec<TeamInfo>;
}

pub enum UpdateTeam<'a> {
    Info {
        id: &'a str,
        name: &'a str,
        description: &'a str,
    },
    MarkAvatarUploaded {
        id: &'a str,
        avatar_version: u32,
    },
}

impl<'a> Oper for UpdateTeam<'a> {
    type Output = ();
}

pub struct ReserveTeamAvatar<'a> {
    pub id: &'a str,
    pub file_ext: &'a str,
}

impl<'a> Oper for ReserveTeamAvatar<'a> {
    type Output = TeamAvatarReservation;
}

pub enum GetTeamInfoExcluded<'a> {
    Id { id: &'a str },
}

impl<'a> Oper for GetTeamInfoExcluded<'a> {
    type Output = TeamInfo;
}

pub struct DeleteTeam<'a> {
    pub id: &'a str,
}

impl<'a> Oper for DeleteTeam<'a> {
    type Output = ();
}

pub struct AllocTeamWorksetIndex<'a> {
    pub id: &'a str,
}

impl<'a> Oper for AllocTeamWorksetIndex<'a> {
    type Output = i32;
}
