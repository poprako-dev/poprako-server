//! Mock implementations of `MemberRepo` and `MemberRepoTransactional` for in-memory testing.

use async_trait::async_trait;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::model::member::{MemberForm, MemberInfo, MemberListSpec};
use crate::model::role::RoleMask;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::step::member::{
    Create, Delete, FindInfoByUserTeamId, GetInfoById, GetInfoExcluded, ListInfos,
    ListInfosByUserIdExcluded, TouchLastActive, UpdateRole, UpdateUserNickname,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_mock::{Mock, MockContext, MockState, MockTransactional, expected, now};
use crate::result::RootError;

impl MemberRepo<MockContext> for Mock {}

impl MemberRepoTransactional<MockContext> for MockTransactional {}

/// Returns [`Some(now)`] when the given role mask has any bits set, used to timestamp role
/// assignments.
fn role_time(role_mask: RoleMask) -> Option<OffsetDateTime> {
    (u32::from(role_mask) != 0).then_some(now())
}

/// Inserts a new member record, rejecting duplicates by id or by the same user+team pair.
fn create_member(state: &mut MockState, form: &MemberForm) -> Result<MemberInfo, RootError> {
    if state.members.iter().any(|member| member.id == form.id) {
        return Err(expected("error-already-exists"));
    }
    if state
        .members
        .iter()
        .any(|member| member.user_id == form.user_id && member.team_id == form.team_id)
    {
        return Err(expected("error-already-exists"));
    }

    let _ = role_time(form.roles);
    let member = MemberInfo {
        id: form.id.clone(),
        user_id: form.user_id.clone(),
        user_nickname: form.user_nickname.clone(),
        team_id: form.team_id.clone(),
        roles: form.roles,
    };
    state.members.push(member.clone());
    Ok(member)
}

fn find_member_by_user_team_id(
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

fn get_member_by_id(state: &MockState, id: &str) -> Result<MemberInfo, RootError> {
    state
        .members
        .iter()
        .find(|member| member.id == id)
        .cloned()
        .ok_or_else(|| expected("error-member-not-found"))
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RootError;

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
    type Error = RootError;

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
impl<'a> Advance<TouchLastActive<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        _: &mut MockContext,
        _: &TouchLastActive<'a>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<ListInfosByUserIdExcluded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

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
    type Error = RootError;

    async fn execute(&self, step: &ListInfos<'a>) -> Result<Vec<MemberInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        let (offset, limit, mut member_infos) = match step.spec {
            MemberListSpec::User {
                owner_id,
                offset,
                limit,
                ..
            } => (
                *offset,
                *limit,
                state
                    .members
                    .iter()
                    .filter(|member_info| member_info.user_id == *owner_id)
                    .cloned()
                    .collect::<Vec<_>>(),
            ),
            MemberListSpec::Team {
                team_id,
                role_bit,
                offset,
                limit,
                ..
            } => (
                *offset,
                *limit,
                state
                    .members
                    .iter()
                    .filter(|member_info| member_info.team_id == *team_id)
                    .filter(|member_info| {
                        role_bit
                            .map(|role_bit| member_info.roles.has_any_role(&[role_bit]))
                            .unwrap_or(true)
                    })
                    .cloned()
                    .collect::<Vec<_>>(),
            ),
        };

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
impl<'a> Execute<FindInfoByUserTeamId<'a>> for Mock {
    type Error = RootError;

    async fn execute(
        &self,
        step: &FindInfoByUserTeamId<'a>,
    ) -> Result<Option<MemberInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        Ok(find_member_by_user_team_id(
            &state,
            step.user_id,
            step.team_id,
        ))
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &GetInfoById<'a>) -> Result<MemberInfo, Self::Error> {
        let state = self.state.lock().unwrap();
        get_member_by_id(&state, step.id)
    }
}

#[async_trait]
impl<'a> Advance<FindInfoByUserTeamId<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &FindInfoByUserTeamId<'a>,
    ) -> Result<Option<MemberInfo>, Self::Error> {
        Ok(find_member_by_user_team_id(
            &context.state,
            step.user_id,
            step.team_id,
        ))
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoExcluded<'a>,
    ) -> Result<MemberInfo, Self::Error> {
        get_member_by_id(&context.state, step.id)
    }
}

#[async_trait]
impl<'a> Advance<UpdateRole<'a>, MockContext> for MockTransactional {
    type Error = RootError;

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
    type Error = RootError;

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
