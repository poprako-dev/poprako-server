//! Mock implementations of `AssignmentRepo` and `AssignmentRepoTransactional`.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::complex::assignment::AssignmentComplex;
use crate::model::assignment::{AssignmentForm, AssignmentInfo};
use crate::part::repo::Execute;
use crate::part::repo::assignment::{AssignmentRepo, AssignmentRepoTransactional};
use crate::part::repo::step::assignment::{
    Create, DeleteByChapter, GetByChapterUserId, PutRoles,
};
use crate::part_impl::repo_mock::{
    Mock, MockContext, MockState, MockTransactional, expected, now,
};
use crate::result::RootError;

impl AssignmentRepo<MockContext> for Mock {}

impl AssignmentRepoTransactional<MockContext> for MockTransactional {}

fn find_assignment(state: &MockState, chapter_id: &str, user_id: &str) -> Option<AssignmentInfo> {
    state
        .assignments
        .iter()
        .find(|assignment_info| {
            assignment_info.chapter_id == chapter_id && assignment_info.user_id == user_id
        })
        .cloned()
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
        roles: form.roles,
        created_at: time,
        updated_at: time,
    };
    state.assignments.push(assignment_info.clone());
    Ok(assignment_info)
}

#[async_trait]
impl<'a> Execute<GetByChapterUserId<'a>> for Mock {
    type Error = RootError;

    async fn execute(
        &self,
        step: &GetByChapterUserId<'a>,
    ) -> Result<Option<AssignmentInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        Ok(find_assignment(&state, step.chapter_id, step.user_id))
    }
}

#[async_trait]
impl<'a> Advance<GetByChapterUserId<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetByChapterUserId<'a>,
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
impl<'a> Advance<DeleteByChapter<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &DeleteByChapter<'a>,
    ) -> Result<(), Self::Error> {
        context
            .state
            .assignments
            .retain(|assignment_info| assignment_info.chapter_id != step.chapter_id);
        Ok(())
    }
}
