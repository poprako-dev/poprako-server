use poprako_orchestra::Oper;

use crate::model::team::{
    TeamAvatarReservation, TeamEntry, TeamInfo, TeamInfoListSpec,
};
use crate::value::image::{ImageExt, ImageHash};

pub struct CreateTeam<'a> {
    pub entry: &'a TeamEntry,
}

impl Oper for CreateTeam<'_> {
    type Output = TeamInfo;
}

pub enum GetTeamInfo<'a> {
    Id { id: &'a str },
}

impl Oper for GetTeamInfo<'_> {
    type Output = TeamInfo;
}

pub struct ListTeamInfos<'a> {
    pub spec: &'a TeamInfoListSpec,
}

impl Oper for ListTeamInfos<'_> {
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
        avatar_key: Option<&'a str>,
        avatar_uploaded: bool,
    },
}

impl Oper for UpdateTeam<'_> {
    type Output = ();
}

pub struct ReserveTeamAvatar<'a> {
    pub id: &'a str,
    pub image_hash: &'a ImageHash,
    pub image_ext: ImageExt,
}

impl Oper for ReserveTeamAvatar<'_> {
    type Output = TeamAvatarReservation;
}

pub enum GetTeamInfoExcluded<'a> {
    Id { id: &'a str },
}

impl Oper for GetTeamInfoExcluded<'_> {
    type Output = TeamInfo;
}

/// Locks a team row.
pub struct LockTeam<'a> {
    pub id: &'a str,
}

impl Oper for LockTeam<'_> {
    type Output = ();
}

pub struct DeleteTeam<'a> {
    pub id: &'a str,
}

impl Oper for DeleteTeam<'_> {
    type Output = ();
}

pub struct AllocTeamWorksetIndex<'a> {
    pub id: &'a str,
}

impl Oper for AllocTeamWorksetIndex<'_> {
    type Output = i32;
}
