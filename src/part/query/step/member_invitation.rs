use poprako_transactional::step::Step;

use crate::model::member_invitation::MemberInvitationInfo;

pub struct GetInfoByCodeExcluded<'a> {
    pub invitation_code: &'a str,
}

impl<'a> Step for GetInfoByCodeExcluded<'a> {
    type Output = MemberInvitationInfo;
}

pub struct MarkPendingAsUsed<'a> {
    pub id: &'a str,
}

impl<'a> Step for MarkPendingAsUsed<'a> {
    type Output = ();
}

pub struct MemberInvitationStep;

impl MemberInvitationStep {
    pub fn get_info_by_code_excluded<'a>(invitation_code: &'a str) -> GetInfoByCodeExcluded<'a> {
        GetInfoByCodeExcluded { invitation_code }
    }

    pub fn mark_pending_as_used<'a>(id: &'a str) -> MarkPendingAsUsed<'a> {
        MarkPendingAsUsed { id }
    }
}
