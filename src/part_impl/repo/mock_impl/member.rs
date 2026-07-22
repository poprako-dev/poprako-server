//! Mock member repository operations for in-memory testing.

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::member::{MemberEntry, MemberInfo, MemberListSpec};
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::part::repo::oper::member::{
    CreateMember, DeleteMember, FindMemberInfo, GetMemberInfo, ListMemberInfos,
    ListMemberInfosExcluded, UpdateMember,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseResult, accept};
use crate::value::member::MemberInclOpt;

fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

fn find_team(state: &MockState, team_id: &str) -> Option<TeamInfo> {
    state
        .teams
        .iter()
        .find(|team_info| team_info.id == team_id)
        .cloned()
}

fn apply_user_incl(
    state: &MockState,
    member_info: &mut MemberInfo,
    include_user: bool,
) {
    //
    member_info.user = None;

    if include_user {
        member_info.user = find_user(state, &member_info.user_id);
    }
}

fn apply_team_incl(
    state: &MockState,
    member_info: &mut MemberInfo,
    include_team: bool,
) {
    //
    member_info.team = None;

    if include_team {
        member_info.team = find_team(state, &member_info.team_id);
    }
}

/// Inserts a new member record, rejecting duplicates by id or by the same user+team pair.
fn create_member(
    state: &mut MockState,
    entry: &MemberEntry,
) -> BaseResult<MemberInfo> {
    //
    if state.members.iter().any(|member| member.id == entry.id) {
        return Err(expected("error-already-exists"));
    }

    if state.members.iter().any(|member| {
        member.user_id == entry.user_id && member.team_id == entry.team_id
    }) {
        return Err(expected("error-already-exists"));
    }

    let member = MemberInfo {
        id: entry.id.clone(),
        user_id: entry.user_id.clone(),
        user_nickname: entry.user_nickname.clone(),
        user_last_active_at: now(),
        team_id: entry.team_id.clone(),
        user: None,
        team: None,
        roles: entry.roles,
    };

    state.members.push(member.clone());

    accept(member)
}

fn find_member_by_user_id_and_team_id(
    state: &MockState,
    user_id: &str,
    team_id: &str,
) -> Option<MemberInfo> {
    state
        .members
        .iter()
        .find(|member| member.user_id == user_id && member.team_id == team_id)
        .cloned()
}

impl<'a> Run<FindMemberInfo<'a>> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &FindMemberInfo<'a>,
    ) -> BaseResult<Option<MemberInfo>> {
        //
        let state = self.state.lock().unwrap();

        match oper {
            FindMemberInfo::UserTeam { user_id, team_id } => accept(
                find_member_by_user_id_and_team_id(&state, user_id, team_id),
            ),
        }
    }
}

fn get_member_by_id(state: &MockState, id: &str) -> BaseResult<MemberInfo> {
    state
        .members
        .iter()
        .find(|member| member.id == id)
        .cloned()
        .ok_or_else(|| expected("error-member-not-found"))
}

fn get_member_info(
    state: &MockState,
    id: &str,
    incls: &[MemberInclOpt],
) -> BaseResult<MemberInfo> {
    //
    let mut member_info = get_member_by_id(state, id)?;

    let include_user = incls.contains(&MemberInclOpt::User);

    let include_team = incls.contains(&MemberInclOpt::Team);

    apply_user_incl(state, &mut member_info, include_user);

    apply_team_incl(state, &mut member_info, include_team);

    accept(member_info)
}

fn list_member_infos(
    state: &MockState,
    spec: &MemberListSpec,
) -> Vec<MemberInfo> {
    //
    let (offset, limit, incls, mut member_infos) = match spec {
        //
        MemberListSpec::User {
            owner_id,
            incl_opt,
            offset,
            limit,
        } => (
            *offset,
            *limit,
            incl_opt,
            state
                .members
                .iter()
                .filter(|member_info| member_info.user_id == *owner_id)
                .cloned()
                .collect::<Vec<_>>(),
        ),

        MemberListSpec::Team {
            team_id,
            fuzzy_nickname,
            role,
            incl_opt,
            offset,
            limit,
        } => (
            *offset,
            *limit,
            incl_opt,
            state
                .members
                .iter()
                .filter(|member_info| member_info.team_id == *team_id)
                .filter(|member_info| {
                    fuzzy_nickname
                        .as_ref()
                        .map(|keyword| {
                            member_info.user_nickname.contains(keyword.as_str())
                        })
                        .unwrap_or(true)
                })
                .filter(|member_info| {
                    role.map(|role| member_info.roles.has_any_role(&[role]))
                        .unwrap_or(true)
                })
                .cloned()
                .collect::<Vec<_>>(),
        ),
    };

    let include_user = incls.contains(&MemberInclOpt::User);

    let include_team = incls.contains(&MemberInclOpt::Team);

    for member_info in &mut member_infos {
        //
        apply_user_incl(state, member_info, include_user);

        apply_team_incl(state, member_info, include_team);
    }

    member_infos.sort_by(|left, right| left.id.cmp(&right.id));

    let offset = offset as usize;

    let limit = limit as usize;

    match offset >= member_infos.len() {
        //
        true => Vec::new(),

        false => {
            //
            let end = std::cmp::min(offset + limit, member_infos.len());

            member_infos[offset..end].to_vec()
        }
    }
}

fn list_member_infos_by_user(
    state: &MockState,
    user_id: &str,
) -> Vec<MemberInfo> {
    state
        .members
        .iter()
        .filter(|member_info| member_info.user_id == user_id)
        .cloned()
        .collect()
}

impl<'a> Run<ListMemberInfos<'a>> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListMemberInfos<'a>,
    ) -> BaseResult<Vec<MemberInfo>> {
        //
        let state = self.state.lock().unwrap();

        match oper {
            //
            ListMemberInfos::Spec { spec } => {
                accept(list_member_infos(&state, spec))
            }

            ListMemberInfos::User { user_id } => {
                accept(list_member_infos_by_user(&state, user_id))
            }
        }
    }
}

impl<'a, 'b> Run<GetMemberInfo<'a, 'b>> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetMemberInfo<'a, 'b>,
    ) -> BaseResult<MemberInfo> {
        //
        let state = self.state.lock().unwrap();

        match oper {
            GetMemberInfo::Id { id, incls } => {
                get_member_info(&state, id, incls)
            }
        }
    }
}

impl<'a> Step<CreateMember<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateMember<'a>,
    ) -> BaseResult<MemberInfo> {
        create_member(&mut context.state, oper.entry)
    }
}

impl<'a> Step<UpdateMember<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateMember<'a>,
    ) -> BaseResult<()> {
        match oper {
            //
            UpdateMember::UserNickname {
                user_id,
                user_nickname,
            } => {
                //
                context
                    .state
                    .members
                    .iter_mut()
                    .filter(|member_info| member_info.user_id == *user_id)
                    .for_each(|member_info| {
                        member_info.user_nickname = user_nickname.to_string();
                    });

                accept(())
            }

            UpdateMember::Role { update } => {
                //
                let member_info = context
                    .state
                    .members
                    .iter_mut()
                    .find(|member_info| member_info.id == update.id)
                    .ok_or_else(|| expected("error-member-not-found"))?;

                member_info.roles = update.roles;

                accept(())
            }
        }
    }
}

impl<'a> Step<ListMemberInfos<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListMemberInfos<'a>,
    ) -> BaseResult<Vec<MemberInfo>> {
        match oper {
            //
            ListMemberInfos::Spec { spec } => {
                accept(list_member_infos(&context.state, spec))
            }

            ListMemberInfos::User { user_id } => {
                accept(list_member_infos_by_user(&context.state, user_id))
            }
        }
    }
}

impl<'a> Step<FindMemberInfo<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &FindMemberInfo<'a>,
    ) -> BaseResult<Option<MemberInfo>> {
        match oper {
            FindMemberInfo::UserTeam { user_id, team_id } => {
                accept(find_member_by_user_id_and_team_id(
                    &context.state,
                    user_id,
                    team_id,
                ))
            }
        }
    }
}

impl<'a, 'b> Step<GetMemberInfo<'a, 'b>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetMemberInfo<'a, 'b>,
    ) -> BaseResult<MemberInfo> {
        match oper {
            GetMemberInfo::Id { id, incls } => {
                get_member_info(&context.state, id, incls)
            }
        }
    }
}

impl<'a> Step<ListMemberInfosExcluded<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListMemberInfosExcluded<'a>,
    ) -> BaseResult<Vec<MemberInfo>> {
        match oper {
            //
            ListMemberInfosExcluded::User { user_id } => {
                accept(list_member_infos_by_user(&context.state, user_id))
            }

            ListMemberInfosExcluded::Team { team_id } => accept(
                context
                    .state
                    .members
                    .iter()
                    .filter(|member_info| member_info.team_id == *team_id)
                    .cloned()
                    .collect(),
            ),
        }
    }
}

impl<'a> Step<DeleteMember<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteMember<'a>,
    ) -> BaseResult<()> {
        //
        let position = context
            .state
            .members
            .iter()
            .position(|member_info| member_info.id == oper.id)
            .ok_or_else(|| expected("error-member-not-found"))?;

        context.state.members.remove(position);

        accept(())
    }
}
