//! Mock implementations of `WorksetRepo` and `WorksetRepoTransactional` for in-memory testing.

use async_trait::async_trait;
use poprako_transactional::advance::Advance;

use crate::model::workset::WorksetInfo;
use crate::part::repo::Execute;
use crate::part::repo::step::workset::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrComicNextIndex, ListByTeamId,
    ListByTeamIdExcluded, UpdateComicCount, UpdateInfo,
};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::part_impl::repo_mock::{Mock, MockContext, MockTransactional, expected, now};
use crate::result::RootError;

impl WorksetRepo<MockContext> for Mock {}

impl WorksetRepoTransactional<MockContext> for MockTransactional {}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &GetInfoById<'a>) -> Result<WorksetInfo, Self::Error> {
        let state = self.state.lock().unwrap();
        state
            .worksets
            .iter()
            .find(|workset| workset.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-workset-not-found"))
    }
}

#[async_trait]
impl<'a> Execute<ListByTeamId<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &ListByTeamId<'a>) -> Result<Vec<WorksetInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        let mut worksets = state
            .worksets
            .iter()
            .filter(|workset| workset.team_id == step.team_id)
            .cloned()
            .collect::<Vec<_>>();
        worksets.sort_by(|left, right| left.index.cmp(&right.index));
        Ok(worksets)
    }
}

#[async_trait]
impl<'a> Execute<UpdateInfo<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &UpdateInfo<'a>) -> Result<(), Self::Error> {
        let mut state = self.state.lock().unwrap();
        let workset = state
            .worksets
            .iter_mut()
            .find(|workset| workset.id == step.update.id)
            .ok_or_else(|| expected("error-workset-not-found"))?;
        workset.name = step.update.name.clone();
        workset.description = step.update.description.clone();
        workset.updated_at = now();
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Create<'a>,
    ) -> Result<WorksetInfo, Self::Error> {
        if context
            .state
            .worksets
            .iter()
            .any(|workset| workset.id == step.form.id)
        {
            return Err(expected("error-already-exists"));
        }

        let time = now();
        let workset = WorksetInfo {
            id: step.form.id.clone(),
            team_id: step.form.team_id.clone(),
            index: step.form.index,
            name: step.form.name.clone(),
            description: step.form.description.clone(),
            comic_count: 0,
            comic_next_index: 0,
            created_at: time,
            updated_at: time,
        };
        context.state.worksets.push(workset.clone());
        Ok(workset)
    }
}

#[async_trait]
impl<'a> Advance<ListByTeamIdExcluded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListByTeamIdExcluded<'a>,
    ) -> Result<Vec<WorksetInfo>, Self::Error> {
        Ok(context
            .state
            .worksets
            .iter()
            .filter(|workset| workset.team_id == step.team_id)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoExcluded<'a>,
    ) -> Result<WorksetInfo, Self::Error> {
        context
            .state
            .worksets
            .iter()
            .find(|workset| workset.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-workset-not-found"))
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
            .worksets
            .iter()
            .position(|workset| workset.id == step.id)
            .ok_or_else(|| expected("error-workset-not-found"))?;

        let deleted_workset_id = context.state.worksets[pos].id.clone();
        context.state.worksets.remove(pos);
        context
            .state
            .comics
            .retain(|comic| comic.workset_id != deleted_workset_id);
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<IncrComicNextIndex<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &IncrComicNextIndex<'a>,
    ) -> Result<i32, Self::Error> {
        let workset = context
            .state
            .worksets
            .iter_mut()
            .find(|workset| workset.id == step.id)
            .ok_or_else(|| expected("error-workset-not-found"))?;
        // Must increment first, then return the new value: frontend expects 1-based index.
        // Never return the pre-increment value.
        workset.comic_next_index += 1;
        workset.updated_at = now();
        Ok(workset.comic_next_index)
    }
}

#[async_trait]
impl<'a> Advance<UpdateComicCount<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &UpdateComicCount<'a>,
    ) -> Result<(), Self::Error> {
        let workset = context
            .state
            .worksets
            .iter_mut()
            .find(|workset| workset.id == step.id)
            .ok_or_else(|| expected("error-workset-not-found"))?;
        workset.comic_count += step.delta;
        workset.updated_at = now();
        Ok(())
    }
}
