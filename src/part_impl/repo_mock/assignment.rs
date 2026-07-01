//! Mock implementations of `AssignmentRepo` and `AssignmentRepoTransactional`.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::model::assignment::{AssignmentForm, AssignmentInfo, AssignmentListSpec};
use crate::model::user::UserInfo;
use crate::part::repo::assignment::{AssignmentRepo, AssignmentRepoTransactional};
use crate::part::repo::step::assignment::{
    Create, Delete, GetInfoByChapterIdAndUserId, GetInfoById, ListInfos, PutRoles,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_mock::{Mock, MockContext, MockState, MockTransactional, expected, now};
use crate::result::RootError;
use crate::value::assignment::AssignmentInclOpt;

impl AssignmentRepo<MockContext> for Mock {}

impl AssignmentRepoTransactional<MockContext> for MockTransactional {}

fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

fn apply_user_incl(state: &MockState, assignment_info: &mut AssignmentInfo, include_user: bool) {
    assignment_info.user = None;

    if include_user {
        assignment_info.user = find_user(state, &assignment_info.user_id);
    }
}

fn find_assignment(state: &MockState, chapter_id: &str, user_id: &str) -> Option<AssignmentInfo> {
    state
        .assignments
        .iter()
        .find(|assignment_info| {
            assignment_info.chapter_id == chapter_id && assignment_info.user_id == user_id
        })
        .cloned()
}

fn get_assignment(state: &MockState, id: &str) -> Result<AssignmentInfo, RootError> {
    state
        .assignments
        .iter()
        .find(|assignment_info| assignment_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-assignment-not-found"))
}

fn list_assignments(state: &MockState, spec: &AssignmentListSpec) -> Vec<AssignmentInfo> {
    let (offset, limit, incl_opt, mut assignment_infos) = match spec {
        AssignmentListSpec::Chapter {
            chapter_id,
            role,
            incl_opt,
            offset,
            limit,
        } => (
            *offset,
            *limit,
            incl_opt,
            state
                .assignments
                .iter()
                .filter(|assignment_info| assignment_info.chapter_id == *chapter_id)
                .filter(|assignment_info| {
                    role.map(|role| assignment_info.roles.has_any_role(&[role]))
                        .unwrap_or(true)
                })
                .cloned()
                .collect::<Vec<_>>(),
        ),
        AssignmentListSpec::User {
            owner_id,
            role,
            incl_opt,
            offset,
            limit,
        } => (
            *offset,
            *limit,
            incl_opt,
            state
                .assignments
                .iter()
                .filter(|assignment_info| assignment_info.user_id == *owner_id)
                .filter(|assignment_info| {
                    role.map(|role| assignment_info.roles.has_any_role(&[role]))
                        .unwrap_or(true)
                })
                .cloned()
                .collect::<Vec<_>>(),
        ),
    };

    let include_user = incl_opt.contains(&AssignmentInclOpt::User);

    for assignment_info in &mut assignment_infos {
        apply_user_incl(state, assignment_info, include_user);
    }

    assignment_infos.sort_by(|left, right| left.id.cmp(&right.id));

    let offset = offset as usize;
    let limit = limit as usize;

    if offset >= assignment_infos.len() {
        return Vec::new();
    }

    let end = std::cmp::min(offset + limit, assignment_infos.len());
    assignment_infos[offset..end].to_vec()
}

fn create_assignment(
    state: &mut MockState,
    form: &AssignmentForm,
) -> Result<AssignmentInfo, RootError> {
    if state
        .assignments
        .iter()
        .any(|assignment_info| assignment_info.id == form.id)
    {
        return Err(expected("error-already-exists"));
    }
    if state.assignments.iter().any(|assignment_info| {
        assignment_info.chapter_id == form.chapter_id && assignment_info.user_id == form.user_id
    }) {
        return Err(expected("error-already-exists"));
    }

    let time = now();
    let assignment_info = AssignmentInfo {
        id: form.id.clone(),
        chapter_id: form.chapter_id.clone(),
        user_id: form.user_id.clone(),
        user: None,
        roles: form.roles,
        created_at: time,
        updated_at: time,
    };
    state.assignments.push(assignment_info.clone());
    Ok(assignment_info)
}

fn delete_assignment_by_id(state: &mut MockState, id: &str) -> Result<(), RootError> {
    let index = state
        .assignments
        .iter()
        .position(|assignment_info| assignment_info.id == id)
        .ok_or_else(|| expected("error-assignment-not-found"))?;

    state.assignments.remove(index);
    Ok(())
}

#[async_trait]
impl<'a> Execute<GetInfoByChapterIdAndUserId<'a>> for Mock {
    type Error = RootError;

    async fn execute(
        &self,
        step: &GetInfoByChapterIdAndUserId<'a>,
    ) -> Result<Option<AssignmentInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        Ok(find_assignment(&state, step.chapter_id, step.user_id))
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &ListInfos<'a>) -> Result<Vec<AssignmentInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        Ok(list_assignments(&state, step.spec))
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &GetInfoById<'a>) -> Result<AssignmentInfo, Self::Error> {
        let state = self.state.lock().unwrap();
        get_assignment(&state, step.id)
    }
}

#[async_trait]
impl<'a> Advance<GetInfoByChapterIdAndUserId<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoByChapterIdAndUserId<'a>,
    ) -> Result<Option<AssignmentInfo>, Self::Error> {
        Ok(find_assignment(
            &context.state,
            step.chapter_id,
            step.user_id,
        ))
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Create<'a>,
    ) -> Result<AssignmentInfo, Self::Error> {
        create_assignment(&mut context.state, step.form)
    }
}

#[async_trait]
impl<'a> Advance<PutRoles<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &PutRoles<'a>,
    ) -> Result<AssignmentInfo, Self::Error> {
        let assignment_info = context
            .state
            .assignments
            .iter_mut()
            .find(|assignment_info| assignment_info.id == step.update.id)
            .ok_or_else(|| expected("error-assignment-not-found"))?;

        assignment_info.roles = step.update.roles;
        assignment_info.updated_at = now();
        Ok(assignment_info.clone())
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
        delete_assignment_by_id(&mut context.state, step.id)
    }
}
