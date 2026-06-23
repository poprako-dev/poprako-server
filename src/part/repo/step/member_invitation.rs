//! Step types for member invitation repository operations.

use poprako_transactional::step::Step;

use crate::model::member_invitation::MemberInvitationInfo;

/// Step that fetches a pending invitation by its code with a pessimistic lock.
pub struct GetInfoByCodeExcluded<'a> {
    pub invitation_code: &'a str,
}

impl<'a> Step for GetInfoByCodeExcluded<'a> {
    type Output = MemberInvitationInfo;
}

/// Step that marks a pending invitation as consumed.
pub struct MarkPendingAsUsed<'a> {
    pub id: &'a str,
}

impl<'a> Step for MarkPendingAsUsed<'a> {
    type Output = ();
}

/// Factory for constructing member invitation repository [`Step`] values.
pub struct MemberInvitationStep;

impl MemberInvitationStep {
    /// Constructs a step to fetch a pending invitation by code with a lock.
    pub fn get_info_by_code_excluded<'a>(invitation_code: &'a str) -> GetInfoByCodeExcluded<'a> {
        GetInfoByCodeExcluded { invitation_code }
    }

    /// Constructs a step to mark a pending invitation as used.
    pub fn mark_pending_as_used<'a>(id: &'a str) -> MarkPendingAsUsed<'a> {
        MarkPendingAsUsed { id }
    }
}
