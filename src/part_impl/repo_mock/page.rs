//! Mock implementations of `PageRepo` and `PageRepoTransactional`.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::model::page::PageInfo;
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::step::page::{ClearImagesByChapter, DeleteByChapterId, ListByChapter};
use crate::part_impl::repo_mock::{Mock, MockContext, MockTransactional, now};
use crate::result::RootError;

impl PageRepo<MockContext> for Mock {}

impl PageRepoTransactional<MockContext> for MockTransactional {}

#[async_trait]
impl<'a> Advance<ListByChapter<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListByChapter<'a>,
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

#[async_trait]
impl<'a> Advance<ClearImagesByChapter<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ClearImagesByChapter<'a>,
    ) -> Result<(), Self::Error> {
        for page_info in &mut context.state.pages {
            if page_info.chapter_id == step.chapter_id {
                page_info.image_key = None;
                page_info.image_uploaded = false;
                page_info.updated_at = now();
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<DeleteByChapterId<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &DeleteByChapterId<'a>,
    ) -> Result<(), Self::Error> {
        context
            .state
            .pages
            .retain(|page_info| page_info.chapter_id != step.chapter_id);
        Ok(())
    }
}
