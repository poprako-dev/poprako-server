use poprako_orchestra::Oper;

use crate::model::member_invitation::{
    MemberInvitationEntry, MemberInvitationInfo, MemberInvitationListSpec,
    MemberInvitationUpdate,
};
use crate::value::member_invitation::MemberInvitationInclOpt;

pub struct CreateMemberInvitation<'a> {
    pub entry: &'a MemberInvitationEntry,
}

impl Oper for CreateMemberInvitation<'_> {
    type Output = MemberInvitationInfo;
}

pub struct ListMemberInvitationInfos<'a> {
    pub spec: &'a MemberInvitationListSpec,
}

impl Oper for ListMemberInvitationInfos<'_> {
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

impl Oper for GetMemberInvitationInfo<'_, '_> {
    type Output = MemberInvitationInfo;
}

pub enum UpdateMemberInvitation<'a> {
    Info { update: &'a MemberInvitationUpdate },
    MarkUsed { id: &'a str },
}

impl Oper for UpdateMemberInvitation<'_> {
    type Output = ();
}

pub enum GetMemberInvitationInfoExcluded<'a> {
    Code { code: &'a str },
}

impl Oper for GetMemberInvitationInfoExcluded<'_> {
    type Output = MemberInvitationInfo;
}

pub struct DeleteMemberInvitation<'a> {
    pub id: &'a str,
}

impl Oper for DeleteMemberInvitation<'_> {
    type Output = ();
}

/// Purges one expired member invitation when it remains pending.
pub struct PurgeExpiredMemberInvitation<'a> {
    pub id: &'a str,
}

impl Oper for PurgeExpiredMemberInvitation<'_> {
    type Output = ();
}
