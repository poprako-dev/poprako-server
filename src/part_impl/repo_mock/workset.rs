use async_trait::async_trait;
use poprako_transactional::advance::Advance;

use crate::model::workset::WorksetInfo;
use crate::part::repo::step::workset::{DeleteCascade, ListByTeamIdExcluded};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::part_impl::repo_mock::{Mock, MockContext, MockTransactional, expected};

impl WorksetRepo<MockContext> for Mock {}

impl WorksetRepoTransactional<MockContext> for MockTransactional {}

#[async_trait]
impl<'a> Advance<ListByTeamIdExcluded<'a>, MockContext> for MockTransactional {
    type Error = crate::result::RootError;

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
impl<'a> Advance<DeleteCascade<'a>, MockContext> for MockTransactional {
    type Error = crate::result::RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &DeleteCascade<'a>,
    ) -> Result<(), Self::Error> {
        let pos = context
            .state
            .worksets
            .iter()
            .position(|workset| workset.id == step.id)
            .ok_or_else(|| expected("error-workset-not-found"))?;
        context.state.worksets.remove(pos);
        Ok(())
    }
}
