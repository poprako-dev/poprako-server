//! Domain models for member invitations.

use crate::value::member_invitation::MemberInvitationInclOpt;

/// Filtering, pagination, and include parameters for listing invitations.
pub struct MemberInvitationListSpec {
    //
    /// The team whose invitations should be listed.
    pub team_id: String,
    /// Optional pending-state filter.
    pub is_pending: Option<bool>,
    /// Additional data to include in each result, such as the inviter user record.
    pub incl_opt: Vec<MemberInvitationInclOpt>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return.
    pub limit: u32,
}
