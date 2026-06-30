//! Mock implementations of `MemberInvitationRepo` and `MemberInvitationRepoTransactional`
//! for in-memory testing.

use async_trait::async_trait;
use poprako_transactional::advance::Advance;

use crate::model::member_invitation::MemberInvitationInfo;
use crate::part::repo::member_invitation::{
    MemberInvitationRepo, MemberInvitationRepoTransactional,
};
use crate::part::repo::step::member_invitation::{
    Create, Delete, GetInfoByCodeExcluded, GetInfoById, ListInfos, MarkPendingAsUsed, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_mock::{Mock, MockContext, MockTransactional, expected};

impl MemberInvitationRepo<MockContext> for Mock {}

impl MemberInvitationRepoTransactional<MockContext> for MockTransactional {}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for Mock {
    type Error = crate::result::RootError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> Result<Vec<MemberInvitationInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        let mut member_invitation_infos = state
            .member_invitations
            .iter()
            .filter(|member_invitation_info| {
                member_invitation_info.team_id == step.team_id
                    && step
                        .pending
                        .is_none_or(|pending| member_invitation_info.pending == pending)
            })
            .cloned()
            .collect::<Vec<_>>();
        member_invitation_infos.sort_by(|left, right| left.id.cmp(&right.id));

        let offset = step.offset as usize;
        let limit = step.limit as usize;
        if offset >= member_invitation_infos.len() {
            return Ok(Vec::new());
        }

        let end = std::cmp::min(offset + limit, member_invitation_infos.len());
        Ok(member_invitation_infos[offset..end].to_vec())
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = crate::result::RootError;

    async fn execute(&self, step: &GetInfoById<'a>) -> Result<MemberInvitationInfo, Self::Error> {
        let state = self.state.lock().unwrap();
        state
            .member_invitations
            .iter()
            .find(|member_invitation_info| member_invitation_info.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-invitation-not-found"))
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = crate::result::RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Create<'a>,
    ) -> Result<MemberInvitationInfo, Self::Error> {
        if context
            .state
            .member_invitations
            .iter()
            .any(|member_invitation_info| member_invitation_info.id == step.form.id)
        {
            return Err(expected("error-already-exists"));
        }
        if context
            .state
            .member_invitations
            .iter()
            .any(|member_invitation_info| {
                member_invitation_info.team_id == step.form.team_id
                    && member_invitation_info.invitee_qid == step.form.invitee_qid
                    && member_invitation_info.pending
            })
        {
            return Err(expected("error-already-exists"));
        }

        let member_invitation_info = MemberInvitationInfo {
            id: step.form.id.clone(),
            team_id: step.form.team_id.clone(),
            invitor_id: step.form.invitor_id.clone(),
            invitee_qid: step.form.invitee_qid.clone(),
            code: step.form.code.clone(),
            pending: true,
            roles: step.form.roles,
        };
        context
            .state
            .member_invitations
            .push(member_invitation_info.clone());
        Ok(member_invitation_info)
    }
}

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
            .find(|invitation| invitation.code == step.code && invitation.pending)
            .cloned()
            .ok_or_else(|| expected("error-no-pending-invitation"))
    }
}

#[async_trait]
impl<'a> Advance<GetInfoById<'a>, MockContext> for MockTransactional {
    type Error = crate::result::RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoById<'a>,
    ) -> Result<MemberInvitationInfo, Self::Error> {
        context
            .state
            .member_invitations
            .iter()
            .find(|member_invitation_info| member_invitation_info.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-invitation-not-found"))
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

#[async_trait]
impl<'a> Advance<UpdateInfo<'a>, MockContext> for MockTransactional {
    type Error = crate::result::RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &UpdateInfo<'a>,
    ) -> Result<(), Self::Error> {
        let member_invitation_info = context
            .state
            .member_invitations
            .iter_mut()
            .find(|member_invitation_info| member_invitation_info.id == step.update.id)
            .ok_or_else(|| expected("error-invitation-not-found"))?;
        member_invitation_info.roles = step.update.roles;
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, MockContext> for MockTransactional {
    type Error = crate::result::RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Delete<'a>,
    ) -> Result<(), Self::Error> {
        let pos = context
            .state
            .member_invitations
            .iter()
            .position(|member_invitation_info| member_invitation_info.id == step.id)
            .ok_or_else(|| expected("error-invitation-not-found"))?;
        context.state.member_invitations.remove(pos);
        Ok(())
    }
}
