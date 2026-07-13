use crate::model::team::TeamAvatarReservation;
use crate::model::team::TeamEntry;
use crate::model::team::TeamInfo;
use poprako_orchestra::Oper;

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
    pub user_id: Option<&'a str>,

    pub offset: u32,
    pub limit: u32,
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

pub struct AllocateTeamWorksetIndex<'a> {
    pub id: &'a str,
}

impl<'a> Oper for AllocateTeamWorksetIndex<'a> {
    type Output = i32;
}
