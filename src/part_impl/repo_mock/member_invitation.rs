//! Mock implementations of `MemberInvitationRepo` and `MemberInvitationRepoTransactional`
//! for in-memory testing.

use async_trait::async_trait;
use poprako_transactional::advance::Advance;

use crate::model::member_invitation::MemberInvitationInfo;
use crate::model::user::UserInfo;
use crate::part::repo::member_invitation::{
    MemberInvitationRepo, MemberInvitationRepoTransactional,
};
use crate::part::repo::step::member_invitation::{
    Create, Delete, GetInfoByCodeExcluded, GetInfoById, ListInfos, MarkPendingAsUsed, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_mock::{Mock, MockContext, MockState, MockTransactional, expected};
use crate::result::RegularError;
use crate::value::member_invitation::MemberInvitationInclOpt;

impl MemberInvitationRepo<MockContext> for Mock {}

impl MemberInvitationRepoTransactional<MockContext> for MockTransactional {}

fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

fn apply_invitor_incl(state: &MockState, info: &mut MemberInvitationInfo, include_invitor: bool) {
    info.invitor = None;
    if include_invitor {
        info.invitor = find_user(state, &info.invitor_id);
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> Result<Vec<MemberInvitationInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        let mut member_invitation_infos = state
            .member_invitations
            .iter()
            .filter(|member_invitation_info| {
                member_invitation_info.team_id == step.spec.team_id
                    && step
                        .spec
                        .pending
                        .is_none_or(|pending| member_invitation_info.pending == pending)
            })
            .cloned()
            .collect::<Vec<_>>();
        member_invitation_infos.sort_by(|left, right| left.id.cmp(&right.id));

        let include_invitor = step
            .spec
            .incl_opt
            .contains(&MemberInvitationInclOpt::Invitor);

        for info in &mut member_invitation_infos {
            apply_invitor_incl(&state, info, include_invitor);
        }

        let offset = step.spec.offset as usize;
        let limit = step.spec.limit as usize;
        if offset >= member_invitation_infos.len() {
            return Ok(Vec::new());
        }

        let end = std::cmp::min(offset + limit, member_invitation_infos.len());
        Ok(member_invitation_infos[offset..end].to_vec())
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = RegularError;

    async fn execute(&self, step: &GetInfoById<'a>) -> Result<MemberInvitationInfo, Self::Error> {
        let state = self.state.lock().unwrap();

        let mut info = state
            .member_invitations
            .iter()
            .find(|i| i.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-invitation-not-found"))?;

        let include_invitor = step.incl_opt.contains(&MemberInvitationInclOpt::Invitor);

        apply_invitor_incl(&state, &mut info, include_invitor);

        Ok(info)
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

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
            invitor: None,
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
    type Error = RegularError;

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
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoById<'a>,
    ) -> Result<MemberInvitationInfo, Self::Error> {
        let mut info = context
            .state
            .member_invitations
            .iter()
            .find(|i| i.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-invitation-not-found"))?;

        let include_invitor = step.incl_opt.contains(&MemberInvitationInclOpt::Invitor);

        apply_invitor_incl(&context.state, &mut info, include_invitor);

        Ok(info)
    }
}

#[async_trait]
impl<'a> Advance<MarkPendingAsUsed<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

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
    type Error = RegularError;

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
    type Error = RegularError;

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
