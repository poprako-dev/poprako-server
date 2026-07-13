use poprako_orchestra::Oper;

use crate::model::member::{
    MemberEntry, MemberInfo, MemberListSpec, MemberRoleUpdate,
};
use crate::value::member::MemberInclOpt;

pub struct CreateMember<'a> {
    pub entry: &'a MemberEntry,
}

impl<'a> Oper for CreateMember<'a> {
    type Output = MemberInfo;
}

pub enum UpdateMember<'a> {
    UserNickname {
        user_id: &'a str,
        user_nickname: &'a str,
    },
    Role {
        update: &'a MemberRoleUpdate,
    },
}

impl<'a> Oper for UpdateMember<'a> {
    type Output = ();
}

pub enum ListMemberInfos<'a> {
    Spec { spec: &'a MemberListSpec },
    User { user_id: &'a str },
}

impl<'a> Oper for ListMemberInfos<'a> {
    type Output = Vec<MemberInfo>;
}

pub enum FindMemberInfo<'a> {
    UserTeam { user_id: &'a str, team_id: &'a str },
}

impl<'a> Oper for FindMemberInfo<'a> {
    type Output = Option<MemberInfo>;
}

pub enum GetMemberInfo<'a, 'b> {
    Id {
        id: &'a str,
        incls: &'b [MemberInclOpt],
    },
}

impl<'a, 'b> Oper for GetMemberInfo<'a, 'b> {
    type Output = MemberInfo;
}

pub enum ListMemberInfosExcluded<'a> {
    User { user_id: &'a str },
}

impl<'a> Oper for ListMemberInfosExcluded<'a> {
    type Output = Vec<MemberInfo>;
}

pub struct DeleteMember<'a> {
    pub id: &'a str,
}

impl<'a> Oper for DeleteMember<'a> {
    type Output = ();
}
