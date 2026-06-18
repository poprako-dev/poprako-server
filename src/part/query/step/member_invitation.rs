use poprako_transactional::step::Step;

use crate::model::member_invitation::MemberInvitationInfo;

pub struct MemberInvitationGetByCodeExcluded<'a> {
    pub invitation_code: &'a str,
}

impl<'a> Step for MemberInvitationGetByCodeExcluded<'a> {
    type Output = MemberInvitationInfo;
}

pub struct MemberInvitationMarkPendingAsUsed<'a> {
    pub id: &'a str,
}

impl<'a> Step for MemberInvitationMarkPendingAsUsed<'a> {
    type Output = ();
}
