use poprako_orchestra::Oper;

use crate::model::assignment_invitation::{
    AssignmentInvitationEntry, AssignmentInvitationInfo,
    AssignmentInvitationListSpec,
};

pub struct CreateAssignmentInvitation<'a> {
    pub entry: &'a AssignmentInvitationEntry,
}

impl<'a> Oper for CreateAssignmentInvitation<'a> {
    type Output = AssignmentInvitationInfo;
}

pub struct ListAssignmentInvitationInfos<'a> {
    pub spec: &'a AssignmentInvitationListSpec,
}

impl<'a> Oper for ListAssignmentInvitationInfos<'a> {
    type Output = Vec<AssignmentInvitationInfo>;
}

pub enum GetAssignmentInvitationInfo<'a> {
    Id { id: &'a str },
}

impl<'a> Oper for GetAssignmentInvitationInfo<'a> {
    type Output = AssignmentInvitationInfo;
}

pub struct GetAssignmentInvitationInfoExcluded<'a> {
    pub code: &'a str,
}

impl<'a> Oper for GetAssignmentInvitationInfoExcluded<'a> {
    type Output = AssignmentInvitationInfo;
}

pub struct MarkAssignmentInvitationUsed<'a> {
    pub id: &'a str,
}

impl<'a> Oper for MarkAssignmentInvitationUsed<'a> {
    type Output = ();
}

pub enum DeleteAssignmentInvitations<'a> {
    Id { id: &'a str },
    Chapter { chapter_id: &'a str },
}

impl<'a> Oper for DeleteAssignmentInvitations<'a> {
    type Output = ();
}
