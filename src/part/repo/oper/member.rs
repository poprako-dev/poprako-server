use poprako_orchestra::Oper;

use crate::model::member::{MemberEntry, MemberInfo, MemberListSpec, MemberRoleUpdate};
use crate::value::member::MemberInclOpt;

pub struct CreateMember<'a> {
    pub entry: &'a MemberEntry,
}

impl Oper for CreateMember<'_> {
    type Output = MemberInfo;
}

pub enum UpdateMember<'a> {
    UserNickname {
        //
        user_id: &'a str,
        user_nickname: &'a str,
    },

    Role {
        update: &'a MemberRoleUpdate,
    },
}

impl Oper for UpdateMember<'_> {
    type Output = ();
}

pub enum ListMemberInfos<'a> {
    Spec { spec: &'a MemberListSpec },

    User { user_id: &'a str },
}

impl Oper for ListMemberInfos<'_> {
    type Output = Vec<MemberInfo>;
}

pub enum FindMemberInfo<'a> {
    UserTeam {
        //
        user_id: &'a str,
        team_id: &'a str,
    },
}

impl Oper for FindMemberInfo<'_> {
    type Output = Option<MemberInfo>;
}

pub enum GetMemberInfo<'a, 'b> {
    Id {
        //
        id: &'a str,
        incls: &'b [MemberInclOpt],
    },
}

impl Oper for GetMemberInfo<'_, '_> {
    type Output = MemberInfo;
}

pub enum ListMemberInfosExcluded<'a> {
    User { user_id: &'a str },

    Team { team_id: &'a str },
}

impl Oper for ListMemberInfosExcluded<'_> {
    type Output = Vec<MemberInfo>;
}

pub struct DeleteMember<'a> {
    pub id: &'a str,
}

impl Oper for DeleteMember<'_> {
    type Output = ();
}
