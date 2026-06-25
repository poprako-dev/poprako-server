//! Mock implementations of `MemberInvitationRepo` and `MemberInvitationRepoTransactional`
//! for in-memory testing.

use async_trait::async_trait;
use poprako_transactional::advance::Advance;

use crate::model::member_invitation::MemberInvitationInfo;
use crate::part::repo::member_invitation::{
    MemberInvitationRepo, MemberInvitationRepoTransactional,
};
use crate::part::repo::step::member_invitation::{GetInfoByCodeExcluded, MarkPendingAsUsed};
use crate::part_impl::repo_mock::{Mock, MockContext, MockTransactional, expected};

impl MemberInvitationRepo<MockContext> for Mock {}

impl MemberInvitationRepoTransactional<MockContext> for MockTransactional {}

#[async_trait]
impl<'a> Advance<GetInfoByCodeExcluded<'a>, MockContext> for MockTransactional {
    type Error = crate::result::RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoByCodeExcluded<'a>,
    ) -> Result<MemberInvitationInfo, Self::Error> {
        context
            .state
            .member_invitations
            .iter()
            .find(|invitation| invitation.code == step.invitation_code && invitation.pending)
            .cloned()
            .ok_or_else(|| expected("error-no-pending-invitation"))
    }
}

#[async_trait]
impl<'a> Advance<MarkPendingAsUsed<'a>, MockContext> for MockTransactional {
    type Error = crate::result::RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &MarkPendingAsUsed<'a>,
    ) -> Result<(), Self::Error> {
        let invitation = context
            .state
            .member_invitations
            .iter_mut()
            .find(|invitation| invitation.id == step.id && invitation.pending)
            .ok_or_else(|| expected("error-invitation-not-found"))?;
        invitation.pending = false;
        Ok(())
    }
}
