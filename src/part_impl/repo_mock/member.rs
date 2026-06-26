//! Mock implementations of `MemberRepo` and `MemberRepoTransactional` for in-memory testing.

use async_trait::async_trait;
use poprako_transactional::advance::Advance;
use time::OffsetDateTime;

use crate::model::member::{MemberForm, MemberInfo};
use crate::part::repo::Execute;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::step::member::{
    Create, Delete, FindByUserTeamId, ListByUserIdExcluded, TouchLastActive, UpdateUserNickname,
};
use crate::part_impl::repo_mock::{Mock, MockContext, MockState, MockTransactional, expected, now};
use crate::result::RootError;

impl MemberRepo<MockContext> for Mock {}

impl MemberRepoTransactional<MockContext> for MockTransactional {}

/// Returns [`Some(now)`] when the given role mask has any bits set, used to timestamp role
/// assignments.
fn role_time(mask: crate::model::role::RoleMask) -> Option<OffsetDateTime> {
    let crate::model::role::RoleMask(bits) = mask;
    (bits != 0).then_some(now())
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

    let _ = role_time(form.role_mask);
    let member = MemberInfo {
        id: form.id.clone(),
        user_id: form.user_id.clone(),
        user_nickname: form.user_nickname.clone(),
        team_id: form.team_id.clone(),
        role_mask: form.role_mask,
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
impl<'a> Advance<ListByUserIdExcluded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListByUserIdExcluded<'a>,
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
impl<'a> Execute<FindByUserTeamId<'a>> for Mock {
    type Error = RootError;

    async fn execute(
        &self,
        step: &FindByUserTeamId<'a>,
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
impl<'a> Advance<FindByUserTeamId<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &FindByUserTeamId<'a>,
    ) -> Result<Option<MemberInfo>, Self::Error> {
        Ok(find_member_by_user_team_id(
            &context.state,
            step.user_id,
            step.team_id,
        ))
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
