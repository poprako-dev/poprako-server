use poprako_orchestra::Oper;

use crate::model::assignment_invitation::{AssignmentInvitationEntry, AssignmentInvitationInfo, AssignmentInvitationListSpec};

pub struct CreateAssignmentInvitation<'a> {
    pub entry: &'a AssignmentInvitationEntry,
}

impl Oper for CreateAssignmentInvitation<'_> {
    type Output = AssignmentInvitationInfo;
}

pub struct ListAssignmentInvitationInfos<'a> {
    pub spec: &'a AssignmentInvitationListSpec,
}

impl Oper for ListAssignmentInvitationInfos<'_> {
    type Output = Vec<AssignmentInvitationInfo>;
}

pub enum GetAssignmentInvitationInfo<'a> {
    Id { id: &'a str },
}

impl Oper for GetAssignmentInvitationInfo<'_> {
    type Output = AssignmentInvitationInfo;
}

pub struct GetAssignmentInvitationInfoExcluded<'a> {
    pub code: &'a str,
}

impl Oper for GetAssignmentInvitationInfoExcluded<'_> {
    type Output = AssignmentInvitationInfo;
}

pub struct MarkAssignmentInvitationUsed<'a> {
    pub id: &'a str,
}

impl Oper for MarkAssignmentInvitationUsed<'_> {
    type Output = ();
}

/// Purges one expired assignment invitation when it remains pending.
pub struct PurgeExpiredAssignmentInvitation<'a> {
    pub id: &'a str,
}

impl Oper for PurgeExpiredAssignmentInvitation<'_> {
    type Output = ();
}

pub enum DeleteAssignmentInvitations<'a> {
    Id { id: &'a str },
    Chapter { chapter_id: &'a str },
}

impl Oper for DeleteAssignmentInvitations<'_> {
    type Output = ();
}
