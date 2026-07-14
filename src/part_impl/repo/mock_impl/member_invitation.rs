//! Mock member-invitation repository operations.

use poprako_orchestra::{Run, Step};

use tracing::instrument;

use crate::model::member_invitation::{
    MemberInvitationEntry, MemberInvitationInfo, MemberInvitationListSpec,
};
use crate::model::user::UserInfo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::oper::member_invitation::{
    CreateMemberInvitation, DeleteMemberInvitation, GetMemberInvitationInfo,
    GetMemberInvitationInfoExcluded, ListMemberInvitationInfos,
    UpdateMemberInvitation,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected,
};
use crate::result::{RegularError, RegularResult};
use crate::value::member_invitation::MemberInvitationInclOpt;

impl MemberInvitationRepo<MockContext> for Mock {}

fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

fn apply_invitor_incl(
    state: &MockState,
    member_invitation_info: &mut MemberInvitationInfo,
    include_invitor: bool,
) {
    //
    member_invitation_info.invitor = None;

    if include_invitor {
        member_invitation_info.invitor =
            find_user(state, &member_invitation_info.invitor_id);
    }
}

fn list_member_invitation_infos(
    state: &MockState,
    spec: &MemberInvitationListSpec,
) -> Vec<MemberInvitationInfo> {
    //
    let mut member_invitation_infos = state
        .member_invitations
        .iter()
        .filter(|member_invitation_info| {
            member_invitation_info.team_id == spec.team_id
                && spec.pending.is_none_or(|pending| {
                    member_invitation_info.pending == pending
                })
        })
        .cloned()
        .collect::<Vec<_>>();

    member_invitation_infos.sort_by(|left, right| left.id.cmp(&right.id));

    let include_invitor =
        spec.incl_opt.contains(&MemberInvitationInclOpt::Invitor);

    for member_invitation_info in &mut member_invitation_infos {
        apply_invitor_incl(state, member_invitation_info, include_invitor);
    }

    let offset = spec.offset as usize;

    let limit = spec.limit as usize;

    match offset >= member_invitation_infos.len() {
        //
        true => Vec::new(),

        false => {
            //
            let end =
                std::cmp::min(offset + limit, member_invitation_infos.len());

            member_invitation_infos[offset..end].to_vec()
        }
    }
}

fn get_member_invitation_info(
    state: &MockState,
    oper: &GetMemberInvitationInfo<'_, '_>,
) -> RegularResult<MemberInvitationInfo> {
    match oper {
        //
        GetMemberInvitationInfo::Id { id, incls } => {
            //
            let mut member_invitation_info = state
                .member_invitations
                .iter()
                .find(|member_invitation_info| member_invitation_info.id == *id)
                .cloned()
                .ok_or_else(|| expected("error-invitation-not-found"))?;

            let include_invitor =
                incls.contains(&MemberInvitationInclOpt::Invitor);

            apply_invitor_incl(
                state,
                &mut member_invitation_info,
                include_invitor,
            );

            Ok(member_invitation_info)
        }

        GetMemberInvitationInfo::Code { code } => state
            .member_invitations
            .iter()
            .find(|member_invitation_info| {
                member_invitation_info.code == *code
                    && member_invitation_info.pending
            })
            .cloned()
            .ok_or_else(|| expected("error-no-pending-invitation")),
    }
}

fn create_member_invitation(
    state: &mut MockState,
    entry: &MemberInvitationEntry,
) -> RegularResult<MemberInvitationInfo> {
    //
    if state
        .member_invitations
        .iter()
        .any(|member_invitation_info| member_invitation_info.id == entry.id)
    {
        return Err(expected("error-already-exists"));
    }

    if state
        .member_invitations
        .iter()
        .any(|member_invitation_info| {
            member_invitation_info.team_id == entry.team_id
                && member_invitation_info.invitee_qid == entry.invitee_qid
                && member_invitation_info.pending
        })
    {
        return Err(expected("error-already-exists"));
    }

    let member_invitation_info = MemberInvitationInfo {
        id: entry.id.clone(),
        team_id: entry.team_id.clone(),
        invitor: None,
        invitor_id: entry.invitor_id.clone(),
        invitee_qid: entry.invitee_qid.clone(),
        code: entry.code.clone(),
        pending: true,
        roles: entry.roles,
    };

    state
        .member_invitations
        .push(member_invitation_info.clone());

    Ok(member_invitation_info)
}

fn update_member_invitation(
    state: &mut MockState,
    oper: &UpdateMemberInvitation<'_>,
) -> RegularResult<()> {
    //
    match oper {
        //
        UpdateMemberInvitation::Info { update } => {
            //
            let member_invitation_info = state
                .member_invitations
                .iter_mut()
                .find(|member_invitation_info| {
                    member_invitation_info.id == update.id
                })
                .ok_or_else(|| expected("error-invitation-not-found"))?;

            member_invitation_info.roles = update.roles;
        }

        UpdateMemberInvitation::MarkUsed { id } => {
            //
            let member_invitation_info = state
                .member_invitations
                .iter_mut()
                .find(|member_invitation_info| {
                    member_invitation_info.id == *id
                        && member_invitation_info.pending
                })
                .ok_or_else(|| expected("error-invitation-not-found"))?;

            member_invitation_info.pending = false;
        }
    }

    Ok(())
}

impl<'a> Run<ListMemberInvitationInfos<'a>> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListMemberInvitationInfos<'a>,
    ) -> RegularResult<Vec<MemberInvitationInfo>> {
        //
        let state = self.state.lock().unwrap();

        Ok(list_member_invitation_infos(&state, oper.spec))
    }
}

impl<'a, 'b> Run<GetMemberInvitationInfo<'a, 'b>> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetMemberInvitationInfo<'a, 'b>,
    ) -> RegularResult<MemberInvitationInfo> {
        //
        let state = self.state.lock().unwrap();

        get_member_invitation_info(&state, oper)
    }
}

impl<'a> Step<CreateMemberInvitation<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateMemberInvitation<'a>,
    ) -> RegularResult<MemberInvitationInfo> {
        create_member_invitation(&mut context.state, oper.entry)
    }
}

impl<'a, 'b> Step<GetMemberInvitationInfo<'a, 'b>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetMemberInvitationInfo<'a, 'b>,
    ) -> RegularResult<MemberInvitationInfo> {
        get_member_invitation_info(&context.state, oper)
    }
}

impl<'a> Step<UpdateMemberInvitation<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateMemberInvitation<'a>,
    ) -> RegularResult<()> {
        update_member_invitation(&mut context.state, oper)
    }
}

impl<'a> Step<GetMemberInvitationInfoExcluded<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetMemberInvitationInfoExcluded<'a>,
    ) -> RegularResult<MemberInvitationInfo> {
        match oper {
            GetMemberInvitationInfoExcluded::Code { code } => {
                //

                get_member_invitation_info(
                    &context.state,
                    &GetMemberInvitationInfo::Code { code },
                )
            }
        }
    }
}

impl<'a> Step<DeleteMemberInvitation<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteMemberInvitation<'a>,
    ) -> RegularResult<()> {
        //
        let position = context
            .state
            .member_invitations
            .iter()
            .position(|member_invitation_info| {
                member_invitation_info.id == oper.id
            })
            .ok_or_else(|| expected("error-invitation-not-found"))?;

        context.state.member_invitations.remove(position);

        Ok(())
    }
}
