use poprako_orchestra::Oper;

use crate::model::member_invitation::{
    MemberInvitationEntry, MemberInvitationInfo, MemberInvitationListSpec,
    MemberInvitationUpdate,
};
use crate::value::member_invitation::MemberInvitationInclOpt;

/// Creates a new member invitation.
pub struct CreateMemberInvitation<'a> {
    /// The invitation entry to insert.
    pub entry: &'a MemberInvitationEntry,
}

impl Oper for CreateMemberInvitation<'_> {
    // The created invitation info.
    type Output = MemberInvitationInfo;
}

/// Lists member invitations matching the given spec.
pub struct ListMemberInvitationInfos<'a> {
    /// The filter and pagination specification.
    pub spec: &'a MemberInvitationListSpec,
}

impl Oper for ListMemberInvitationInfos<'_> {
    // List of matching invitation infos.
    type Output = Vec<MemberInvitationInfo>;
}

/// Retrieves a single invitation info by ID or code.
pub enum GetMemberInvitationInfo<'a, 'b> {
    /// Retrieves by invitation ID.
    Id {
        //
        /// The invitation ID.
        id: &'a str,
        /// Which relations to include in the response.
        incls: &'b [MemberInvitationInclOpt],
    },

    /// Retrieves by invitation code.
    Code {
        /// The invitation code.
        code: &'a str,
    },
}

impl Oper for GetMemberInvitationInfo<'_, '_> {
    // The retrieved invitation info.
    type Output = MemberInvitationInfo;
}

/// Updates a member invitation's fields or marks it as used.
pub enum UpdateMemberInvitation<'a> {
    /// Updates the invitation fields.
    Info {
        /// The update payload.
        update: &'a MemberInvitationUpdate,
    },

    /// Marks the invitation as used.
    MarkUsed {
        /// The invitation ID.
        id: &'a str,
    },
}

impl Oper for UpdateMemberInvitation<'_> {
    // Unit on success.
    type Output = ();
}

/// Retrieves invitation info by code with excluded fields omitted.
pub enum GetMemberInvitationInfoExcluded<'a> {
    /// Retrieves by invitation code.
    Code {
        /// The invitation code.
        code: &'a str,
    },
}

impl Oper for GetMemberInvitationInfoExcluded<'_> {
    // The retrieved invitation info with excluded fields omitted.
    type Output = MemberInvitationInfo;
}

/// Deletes a member invitation by ID.
pub struct DeleteMemberInvitation<'a> {
    /// The invitation ID to delete.
    pub id: &'a str,
}

impl Oper for DeleteMemberInvitation<'_> {
    // Unit on success.
    type Output = ();
}

/// Purges one expired member invitation when it remains pending.
pub struct PurgeExpiredMemberInvitation<'a> {
    /// The invitation ID to purge.
    pub id: &'a str,
}

impl Oper for PurgeExpiredMemberInvitation<'_> {
    // Unit on success.
    type Output = ();
}
