use poprako_orchestra::Oper;

use crate::model::member_invitation::{
    MemberInvitationEntry, MemberInvitationInfo, MemberInvitationListSpec,
    MemberInvitationUpdate,
};
use crate::value::member_invitation::MemberInvitationInclOpt;

pub struct CreateMemberInvitation<'a> {
    pub entry: &'a MemberInvitationEntry,
}

impl<'a> Oper for CreateMemberInvitation<'a> {
    type Output = MemberInvitationInfo;
}

pub struct ListMemberInvitationInfos<'a> {
    pub spec: &'a MemberInvitationListSpec,
}

impl<'a> Oper for ListMemberInvitationInfos<'a> {
    type Output = Vec<MemberInvitationInfo>;
}

pub enum GetMemberInvitationInfo<'a, 'b> {
    Id {
        id: &'a str,
        incls: &'b [MemberInvitationInclOpt],
    },
    Code {
        code: &'a str,
    },
}

impl<'a, 'b> Oper for GetMemberInvitationInfo<'a, 'b> {
    type Output = MemberInvitationInfo;
}

pub enum UpdateMemberInvitation<'a> {
    Info { update: &'a MemberInvitationUpdate },
    MarkUsed { id: &'a str },
}

impl<'a> Oper for UpdateMemberInvitation<'a> {
    type Output = ();
}

pub enum GetMemberInvitationInfoExcluded<'a> {
    Code { code: &'a str },
}

impl<'a> Oper for GetMemberInvitationInfoExcluded<'a> {
    type Output = MemberInvitationInfo;
}

pub struct DeleteMemberInvitation<'a> {
    pub id: &'a str,
}

impl<'a> Oper for DeleteMemberInvitation<'a> {
    type Output = ();
}

/// Purges one expired member invitation when it remains pending.
pub struct PurgeExpiredMemberInvitation<'a> {
    pub id: &'a str,
}

impl<'a> Oper for PurgeExpiredMemberInvitation<'a> {
    type Output = ();
}
