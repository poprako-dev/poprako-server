//! Mock implementations of assignment invitation repository opers.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::model::assignment_invitation_model;
use crate::part::repo::assignment_invitation::{
    AssignmentInvitationRepo, AssignmentInvitationRepoTransactional,
};
use crate::part::repo::step::assignment_invitation::{
    Create, Delete, DeleteByChapterId, GetInfoByCodeExcluded, GetInfoById,
    ListInfos, MarkPendingAsUsed,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockTransactional, expected, now,
};
use crate::result::RegularError;

impl AssignmentInvitationRepo<MockContext> for Mock {}

impl AssignmentInvitationRepoTransactional<MockContext> for MockTransactional {}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> Result<Vec<assignment_invitation_model::Info>, Self::Error> {
        //
        let state = self.state.lock().unwrap();

        let mut assignment_invitation_infos = state
            .assignment_invitations
            .iter()
            .filter(|assignment_invitation_info| {
                assignment_invitation_info.chapter_id == step.chapter_id
                    && step.pending.is_none_or(|pending| {
                        assignment_invitation_info.pending == pending
                    })
            })
            .cloned()
            .collect::<Vec<_>>();

        assignment_invitation_infos.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });

        let offset = step.offset as usize;

        let limit = step.limit as usize;

        if offset >= assignment_invitation_infos.len() {
            return Ok(Vec::new());
        }

        let end =
            std::cmp::min(offset + limit, assignment_invitation_infos.len());

        Ok(assignment_invitation_infos[offset..end].to_vec())
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> Result<assignment_invitation_model::Info, Self::Error> {
        //
        let state = self.state.lock().unwrap();

        state
            .assignment_invitations
            .iter()
            .find(|assignment_invitation_info| {
                assignment_invitation_info.id == step.id
            })
            .cloned()
            .ok_or_else(|| expected("error-invitation-not-found"))
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Create<'a>,
    ) -> Result<assignment_invitation_model::Info, Self::Error> {
        //
        if context.state.assignment_invitations.iter().any(
            |assignment_invitation_info| {
                assignment_invitation_info.id == step.form.id
            },
        ) {
            return Err(expected("error-already-exists"));
        }

        if context.state.assignment_invitations.iter().any(
            |assignment_invitation_info| {
                assignment_invitation_info.chapter_id == step.form.chapter_id
                    && assignment_invitation_info.invitee_qid
                        == step.form.invitee_qid
                    && assignment_invitation_info.pending
            },
        ) {
            return Err(expected("error-already-exists"));
        }

        let time = now();

        let assignment_invitation_info = assignment_invitation_model::Info {
            id: step.form.id.clone(),
            chapter_id: step.form.chapter_id.clone(),
            inviter_id: step.form.inviter_id.clone(),
            invitee_qid: step.form.invitee_qid.clone(),
            code: step.form.code.clone(),
            pending: true,
            roles: step.form.roles,
            created_at: time,
            updated_at: time,
        };

        context
            .state
            .assignment_invitations
            .push(assignment_invitation_info.clone());

        Ok(assignment_invitation_info)
    }
}

#[async_trait]
impl<'a> Advance<GetInfoById<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoById<'a>,
    ) -> Result<assignment_invitation_model::Info, Self::Error> {
        context
            .state
            .assignment_invitations
            .iter()
            .find(|assignment_invitation_info| {
                assignment_invitation_info.id == step.id
            })
            .cloned()
            .ok_or_else(|| expected("error-invitation-not-found"))
    }
}

#[async_trait]
impl<'a> Advance<GetInfoByCodeExcluded<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoByCodeExcluded<'a>,
    ) -> Result<assignment_invitation_model::Info, Self::Error> {
        context
            .state
            .assignment_invitations
            .iter()
            .find(|invitation| {
                invitation.code == step.code && invitation.pending
            })
            .cloned()
            .ok_or_else(|| expected("error-no-pending-invitation"))
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
        //
        let invitation = context
            .state
            .assignment_invitations
            .iter_mut()
            .find(|invitation| invitation.id == step.id && invitation.pending)
            .ok_or_else(|| expected("error-invitation-not-found"))?;

        invitation.pending = false;

        invitation.updated_at = now();

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
        //
        let pos = context
            .state
            .assignment_invitations
            .iter()
            .position(|assignment_invitation_info| {
                assignment_invitation_info.id == step.id
            })
            .ok_or_else(|| expected("error-invitation-not-found"))?;

        context.state.assignment_invitations.remove(pos);

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<DeleteByChapterId<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &DeleteByChapterId<'a>,
    ) -> Result<(), Self::Error> {
        //
        context
            .state
            .assignment_invitations
            .retain(|inv| inv.chapter_id != step.chapter_id);

        Ok(())
    }
}
