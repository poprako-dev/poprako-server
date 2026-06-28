//! Mock implementations of `PageRepo` and `PageRepoTransactional`.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::model::page::PageInfo;
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::step::page::ListInfosByChapter;
use crate::part_impl::repo_mock::{Mock, MockContext, MockTransactional};
use crate::result::RootError;

impl PageRepo<MockContext> for Mock {}

impl PageRepoTransactional<MockContext> for MockTransactional {}

#[async_trait]
impl<'a> Advance<ListInfosByChapter<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListInfosByChapter<'a>,
    ) -> Result<Vec<PageInfo>, Self::Error> {
        Ok(context
            .state
            .pages
            .iter()
            .filter(|page_info| page_info.chapter_id == step.chapter_id)
            .cloned()
            .collect())
    }
}
