//! Mock implementations of `MemberRepo` and `MemberRepoTransactional` for in-memory testing.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::model::member::{MemberForm, MemberInfo, MemberListSpec};
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::step::member::{
    Create, Delete, FindInfoByUserIdAndTeamId, GetInfoById, ListInfos,
    ListInfosByUserIdExcluded, UpdateRole, UpdateUserNickname,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, MockTransactional, expected, now,
};
use crate::result::{RegularError, RegularResult};
use crate::value::member::MemberInclOpt;

impl MemberRepo<MockContext> for Mock {}

impl MemberRepoTransactional<MockContext> for MockTransactional {}

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
    form: &MemberForm,
) -> RegularResult<MemberInfo> {
    //
    if state.members.iter().any(|member| member.id == form.id) {
        return Err(expected("error-already-exists"));
    }

    if state.members.iter().any(|member| {
        member.user_id == form.user_id && member.team_id == form.team_id
    }) {
        return Err(expected("error-already-exists"));
    }

    let member = MemberInfo {
        id: form.id.clone(),
        user_id: form.user_id.clone(),
        user_nickname: form.user_nickname.clone(),
        user_last_active_at: now(),
        team_id: form.team_id.clone(),
        user: None,
        team: None,
        roles: form.roles,
    };

    state.members.push(member.clone());

    Ok(member)
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

fn get_member_by_id(state: &MockState, id: &str) -> RegularResult<MemberInfo> {
    state
        .members
        .iter()
        .find(|member| member.id == id)
        .cloned()
        .ok_or_else(|| expected("error-member-not-found"))
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Create<'a>,
    ) -> Result<MemberInfo, Self::Error> {
        create_member(&mut context.state, step.form)
    }
}

#[async_trait]
impl<'a> Advance<UpdateUserNickname<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &UpdateUserNickname<'a>,
    ) -> Result<(), Self::Error> {
        for member in &mut context.state.members {
            if member.user_id == step.user_id {
                member.user_nickname = step.user_nickname.to_string();
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<ListInfosByUserIdExcluded<'a>, MockContext>
    for MockTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListInfosByUserIdExcluded<'a>,
    ) -> Result<Vec<MemberInfo>, Self::Error> {
        Ok(context
            .state
            .members
            .iter()
            .filter(|member| member.user_id == step.user_id)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> Result<Vec<MemberInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        let (offset, limit, incl_opt, mut member_infos) = match step.spec {
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
                            .map(|kw| {
                                member_info.user_nickname.contains(kw.as_str())
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

        let include_user = incl_opt.contains(&MemberInclOpt::User);
        let include_team = incl_opt.contains(&MemberInclOpt::Team);

        for member_info in &mut member_infos {
            apply_user_incl(&state, member_info, include_user);
            apply_team_incl(&state, member_info, include_team);
        }

        member_infos.sort_by(|left, right| left.id.cmp(&right.id));

        let offset = offset as usize;
        let limit = limit as usize;

        if offset >= member_infos.len() {
            return Ok(Vec::new());
        }

        let end = std::cmp::min(offset + limit, member_infos.len());
        Ok(member_infos[offset..end].to_vec())
    }
}

#[async_trait]
impl<'a> Execute<FindInfoByUserIdAndTeamId<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &FindInfoByUserIdAndTeamId<'a>,
    ) -> Result<Option<MemberInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        Ok(find_member_by_user_id_and_team_id(
            &state,
            step.user_id,
            step.team_id,
        ))
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> Result<MemberInfo, Self::Error> {
        let state = self.state.lock().unwrap();

        let mut info = get_member_by_id(&state, step.id)?;

        let include_user = step.incl_opt.contains(&MemberInclOpt::User);
        let include_team = step.incl_opt.contains(&MemberInclOpt::Team);

        apply_user_incl(&state, &mut info, include_user);
        apply_team_incl(&state, &mut info, include_team);

        Ok(info)
    }
}

#[async_trait]
impl<'a> Advance<FindInfoByUserIdAndTeamId<'a>, MockContext>
    for MockTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &FindInfoByUserIdAndTeamId<'a>,
    ) -> Result<Option<MemberInfo>, Self::Error> {
        Ok(find_member_by_user_id_and_team_id(
            &context.state,
            step.user_id,
            step.team_id,
        ))
    }
}

#[async_trait]
impl<'a> Advance<UpdateRole<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &UpdateRole<'a>,
    ) -> Result<(), Self::Error> {
        let member_info = context
            .state
            .members
            .iter_mut()
            .find(|member_info| member_info.id == step.member_role_update.id)
            .ok_or_else(|| expected("error-member-not-found"))?;

        member_info.roles = step.member_role_update.roles;
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
            .members
            .iter()
            .position(|member| member.id == step.id)
            .ok_or_else(|| expected("error-member-not-found"))?;
        context.state.members.remove(pos);
        Ok(())
    }
}
