//! Mock implementations of `ChapterRepo` and `ChapterRepoTransactional`.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::model::chapter::{ChapterForm, ChapterInfo};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::step::chapter::{
    AdjustUnitCounters, Create, Delete, FindPinnedInfoByComicId, GetInfoById, GetInfoByIdExcluded,
    ListAllInfosByComicIdExcluded, ListInfosByComicId, ListInfosByComicIdExcluded, SetPageCounters,
    UnpinOthers, UpdateInfo, UpdateStage,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_mock::{Mock, MockContext, MockState, MockTransactional, expected, now};
use crate::result::RootError;
use crate::value::chapter::WorkflowStageMask;

impl ChapterRepo<MockContext> for Mock {}

impl ChapterRepoTransactional<MockContext> for MockTransactional {}

fn get_chapter_by_id(state: &MockState, id: &str) -> Result<ChapterInfo, RootError> {
    state
        .chapters
        .iter()
        .find(|chapter_info| chapter_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-chapter-not-found"))
}

fn list_chapters(state: &MockState, comic_id: &str, offset: u64, limit: u64) -> Vec<ChapterInfo> {
    let chapter_infos = list_all_chapters(state, comic_id);

    let offset = offset as usize;
    let limit = limit as usize;
    if offset >= chapter_infos.len() {
        return Vec::new();
    }

    let end = std::cmp::min(offset + limit, chapter_infos.len());
    chapter_infos[offset..end].to_vec()
}

fn list_all_chapters(state: &MockState, comic_id: &str) -> Vec<ChapterInfo> {
    let mut chapter_infos = state
        .chapters
        .iter()
        .filter(|chapter_info| chapter_info.comic_id == comic_id)
        .cloned()
        .collect::<Vec<_>>();
    chapter_infos.sort_by(|left, right| right.index.cmp(&left.index));

    chapter_infos
}

fn create_chapter(state: &mut MockState, form: &ChapterForm) -> Result<ChapterInfo, RootError> {
    if state
        .chapters
        .iter()
        .any(|chapter_info| chapter_info.id == form.id)
    {
        return Err(expected("error-already-exists"));
    }

    let time = now();
    let chapter_info = ChapterInfo {
        id: form.id.clone(),
        comic_id: form.comic_id.clone(),
        is_pinned: form.is_pinned,
        index: form.index,
        subtitle: form.subtitle.clone(),
        page_count: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: WorkflowStageMask::try_from(0u32).ok().unwrap(),
        creator_id: form.creator_id.clone(),
        created_at: time,
        updated_at: time,
    };

    state.chapters.push(chapter_info.clone());
    Ok(chapter_info)
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &GetInfoById<'a>) -> Result<ChapterInfo, Self::Error> {
        let state = self.state.lock().unwrap();
        get_chapter_by_id(&state, step.id)
    }
}

#[async_trait]
impl<'a> Execute<ListInfosByComicId<'a>> for Mock {
    type Error = RootError;

    async fn execute(
        &self,
        step: &ListInfosByComicId<'a>,
    ) -> Result<Vec<ChapterInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        Ok(list_chapters(
            &state,
            step.comic_id,
            step.offset,
            step.limit,
        ))
    }
}

#[async_trait]
impl<'a> Execute<FindPinnedInfoByComicId<'a>> for Mock {
    type Error = RootError;

    async fn execute(
        &self,
        step: &FindPinnedInfoByComicId<'a>,
    ) -> Result<Option<ChapterInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        Ok(state
            .chapters
            .iter()
            .find(|chapter_info| chapter_info.comic_id == step.comic_id && chapter_info.is_pinned)
            .cloned())
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Create<'a>,
    ) -> Result<ChapterInfo, Self::Error> {
        create_chapter(&mut context.state, step.form)
    }
}

#[async_trait]
impl<'a> Advance<GetInfoById<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoById<'a>,
    ) -> Result<ChapterInfo, Self::Error> {
        get_chapter_by_id(&context.state, step.id)
    }
}

#[async_trait]
impl<'a> Advance<GetInfoByIdExcluded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoByIdExcluded<'a>,
    ) -> Result<ChapterInfo, Self::Error> {
        get_chapter_by_id(&context.state, step.id)
    }
}

#[async_trait]
impl<'a> Advance<ListInfosByComicIdExcluded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListInfosByComicIdExcluded<'a>,
    ) -> Result<Vec<ChapterInfo>, Self::Error> {
        Ok(list_chapters(
            &context.state,
            step.comic_id,
            step.offset,
            step.limit,
        ))
    }
}

#[async_trait]
impl<'a> Advance<ListAllInfosByComicIdExcluded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListAllInfosByComicIdExcluded<'a>,
    ) -> Result<Vec<ChapterInfo>, Self::Error> {
        Ok(list_all_chapters(&context.state, step.comic_id))
    }
}

#[async_trait]
impl<'a> Advance<FindPinnedInfoByComicId<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &FindPinnedInfoByComicId<'a>,
    ) -> Result<Option<ChapterInfo>, Self::Error> {
        Ok(context
            .state
            .chapters
            .iter()
            .find(|chapter_info| chapter_info.comic_id == step.comic_id && chapter_info.is_pinned)
            .cloned())
    }
}

#[async_trait]
impl<'a> Advance<UpdateInfo<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &UpdateInfo<'a>,
    ) -> Result<(), Self::Error> {
        let chapter_info = context
            .state
            .chapters
            .iter_mut()
            .find(|chapter_info| chapter_info.id == step.update.id)
            .ok_or_else(|| expected("error-chapter-not-found"))?;

        if let Some(subtitle) = &step.update.subtitle {
            chapter_info.subtitle = subtitle.clone();
        }
        if let Some(is_pinned) = step.update.pin {
            chapter_info.is_pinned = is_pinned;
        }
        chapter_info.updated_at = now();
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<UpdateStage<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &UpdateStage<'a>,
    ) -> Result<(), Self::Error> {
        let chapter_info = context
            .state
            .chapters
            .iter_mut()
            .find(|chapter_info| chapter_info.id == step.update.id)
            .ok_or_else(|| expected("error-chapter-not-found"))?;

        chapter_info.stages = step.update.stages;

        chapter_info.updated_at = now();
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<SetPageCounters<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &SetPageCounters<'a>,
    ) -> Result<(), Self::Error> {
        let chapter_info = context
            .state
            .chapters
            .iter_mut()
            .find(|chapter_info| chapter_info.id == step.id)
            .ok_or_else(|| expected("error-chapter-not-found"))?;

        chapter_info.page_count = step.page_count;
        chapter_info.total_unit_count = step.total_unit_count;
        chapter_info.translated_unit_count = step.translated_unit_count;
        chapter_info.proofread_unit_count = step.proofread_unit_count;
        chapter_info.updated_at = now();

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<AdjustUnitCounters<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &AdjustUnitCounters<'a>,
    ) -> Result<(), Self::Error> {
        let chapter_info = context
            .state
            .chapters
            .iter_mut()
            .find(|chapter_info| chapter_info.id == step.id)
            .ok_or_else(|| expected("error-chapter-not-found"))?;

        chapter_info.total_unit_count += step.delta.total_unit_count;
        chapter_info.translated_unit_count += step.delta.translated_unit_count;
        chapter_info.proofread_unit_count += step.delta.proofread_unit_count;
        chapter_info.updated_at = now();

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<UnpinOthers<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &UnpinOthers<'a>,
    ) -> Result<(), Self::Error> {
        for chapter_info in &mut context.state.chapters {
            if chapter_info.comic_id == step.comic_id && chapter_info.id != step.excluded_id {
                chapter_info.is_pinned = false;
                chapter_info.updated_at = now();
            }
        }
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
        let position = context
            .state
            .chapters
            .iter()
            .position(|chapter_info| chapter_info.id == step.id)
            .ok_or_else(|| expected("error-chapter-not-found"))?;
        context.state.chapters.remove(position);
        context
            .state
            .pages
            .retain(|page_info| page_info.chapter_id != step.id);
        let page_ids = context
            .state
            .pages
            .iter()
            .map(|page_info| page_info.id.clone())
            .collect::<Vec<_>>();
        context
            .state
            .units
            .retain(|unit_info| page_ids.contains(&unit_info.page_id));
        context
            .state
            .assignments
            .retain(|assignment_info| assignment_info.chapter_id != step.id);
        Ok(())
    }
}
