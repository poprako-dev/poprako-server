//! Mock implementations of `ChapterRepo` and `ChapterRepoTransactional`.

use std::collections::HashMap;

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::model::chapter::{ChapterForm, ChapterInfo};
use crate::model::comic::ComicInfo;
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::model::workset::WorksetInfo;
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::step::chapter::{
    AdjustUnitCounters, Create, Delete, FindPinnedInfoByComicId, GetInfoById, GetInfoByIdExcluded,
    ListAllInfosByComicIdExcluded, ListInfos, ListPinnedInfosByComicIds, SetPageCounters,
    UnpinOthers, UpdateInfo, UpdateStage,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_mock::{Mock, MockContext, MockState, MockTransactional, expected, now};
use crate::result::{RegularError, RegularResult};
use crate::value::chapter::{ChapterInclOpt, StageMask};
use crate::value::incl::expand_incl_opts;

impl ChapterRepo<MockContext> for Mock {}

impl ChapterRepoTransactional<MockContext> for MockTransactional {}

fn get_chapter_by_id(
    state: &MockState,
    id: &str,
    incl_opt: &[ChapterInclOpt],
) -> RegularResult<ChapterInfo> {
    let mut chapter_info = state
        .chapters
        .iter()
        .find(|chapter_info| chapter_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-chapter-not-found"))?;

    apply_chapter_incls(state, &mut chapter_info, incl_opt);

    Ok(chapter_info)
}

// fn list_chapters(state: &MockState, comic_id: &str, offset: u64, limit: u64) -> Vec<ChapterInfo> {
//     let chapter_infos = list_all_chapters(state, comic_id);
//
//     let offset = offset as usize;
//     let limit = limit as usize;
//     if offset >= chapter_infos.len() {
//         return Vec::new();
//     }
//
//     let end = std::cmp::min(offset + limit, chapter_infos.len());
//     chapter_infos[offset..end].to_vec()
// }

fn list_all_chapters(state: &MockState, comic_id: &str) -> Vec<ChapterInfo> {
    let mut chapter_infos = state
        .chapters
        .iter()
        .filter(|chapter_info| chapter_info.comic_id == comic_id)
        .cloned()
        .collect::<Vec<_>>();
    chapter_infos.sort_by_key(|right| std::cmp::Reverse(right.index));

    chapter_infos
}

fn create_chapter(state: &mut MockState, form: &ChapterForm) -> RegularResult<ChapterInfo> {
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
        comic: None,
        is_pinned: form.is_pinned,
        index: form.index,
        subtitle: form.subtitle.clone(),
        page_count: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: StageMask::try_from(0u32).ok().unwrap(),
        creator_id: form.creator_id.clone(),
        creator: None,
        created_at: time,
        updated_at: time,
    };

    state.chapters.push(chapter_info.clone());
    Ok(chapter_info)
}

fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

fn find_comic(state: &MockState, comic_id: &str) -> Option<ComicInfo> {
    let mut comic_info = state
        .comics
        .iter()
        .find(|comic_info| comic_info.id == comic_id)
        .cloned()?;

    comic_info.workset = None;
    comic_info.team = None;
    comic_info.creator = None;

    Some(comic_info)
}

fn find_workset(state: &MockState, workset_id: &str) -> Option<WorksetInfo> {
    state
        .worksets
        .iter()
        .find(|workset_info| workset_info.id == workset_id)
        .cloned()
}

fn find_team_for_workset(state: &MockState, workset_info: &WorksetInfo) -> Option<TeamInfo> {
    state
        .teams
        .iter()
        .find(|team_info| team_info.id == workset_info.team_id)
        .cloned()
}

fn apply_creator_incl(state: &MockState, chapter_info: &mut ChapterInfo, include_creator: bool) {
    chapter_info.creator = None;
    if include_creator {
        chapter_info.creator = find_user(state, &chapter_info.creator_id);
    }
}

fn apply_comic_incl(state: &MockState, chapter_info: &mut ChapterInfo, include_comic: bool) {
    chapter_info.comic = None;
    if include_comic {
        chapter_info.comic = find_comic(state, &chapter_info.comic_id);
    }
}

fn apply_comic_workset_incl(
    state: &MockState,
    chapter_info: &mut ChapterInfo,
    include_workset: bool,
) {
    if !include_workset {
        return;
    }

    let Some(comic_info) = &mut chapter_info.comic else {
        return;
    };

    comic_info.workset = find_workset(state, &comic_info.workset_id);
}

fn apply_comic_workset_team_incl(
    state: &MockState,
    chapter_info: &mut ChapterInfo,
    include_team: bool,
) {
    if !include_team {
        return;
    }

    let Some(comic_info) = &mut chapter_info.comic else {
        return;
    };

    let Some(workset_info) = &comic_info.workset else {
        return;
    };

    comic_info.team = find_team_for_workset(state, workset_info);
}

fn apply_comic_creator_incl(
    state: &MockState,
    chapter_info: &mut ChapterInfo,
    include_creator: bool,
) {
    if !include_creator {
        return;
    }

    let Some(comic_info) = &mut chapter_info.comic else {
        return;
    };

    comic_info.creator = find_user(state, &comic_info.creator_id);
}

fn apply_chapter_incls(
    state: &MockState,
    chapter_info: &mut ChapterInfo,
    incl_opt: &[ChapterInclOpt],
) {
    chapter_info.comic = None;
    chapter_info.creator = None;

    for incl_opt in expand_incl_opts(incl_opt) {
        match incl_opt {
            ChapterInclOpt::Comic => apply_comic_incl(state, chapter_info, true),
            ChapterInclOpt::ComicWorkset => apply_comic_workset_incl(state, chapter_info, true),
            ChapterInclOpt::ComicWorksetTeam => {
                apply_comic_workset_team_incl(state, chapter_info, true)
            }
            ChapterInclOpt::ComicCreator => apply_comic_creator_incl(state, chapter_info, true),
            ChapterInclOpt::Creator => apply_creator_incl(state, chapter_info, true),
        }
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for Mock {
    type Error = RegularError;

    async fn execute(&self, step: &ListInfos<'a>) -> Result<Vec<ChapterInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        let mut chapters = list_all_chapters(&state, &step.spec.comic_id);

        for chapter in &mut chapters {
            apply_chapter_incls(&state, chapter, &step.spec.incl_opt);
        }

        let offset = step.spec.offset as usize;
        let limit = step.spec.limit as usize;

        if offset >= chapters.len() {
            return Ok(Vec::new());
        }

        let end = std::cmp::min(offset + limit, chapters.len());
        Ok(chapters[offset..end].to_vec())
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = RegularError;

    async fn execute(&self, step: &GetInfoById<'a>) -> Result<ChapterInfo, Self::Error> {
        let state = self.state.lock().unwrap();

        get_chapter_by_id(&state, step.id, step.incl_opt)
    }
}

// #[async_trait]
// impl<'a> Execute<ListInfosByComicId<'a>> for Mock {
//     ...
// }

#[async_trait]
impl<'a> Execute<FindPinnedInfoByComicId<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &FindPinnedInfoByComicId<'a>,
    ) -> Result<Option<ChapterInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        let mut chapter_info = state
            .chapters
            .iter()
            .find(|chapter_info| chapter_info.comic_id == step.comic_id && chapter_info.is_pinned)
            .cloned();

        if let Some(chapter_info) = &mut chapter_info {
            apply_chapter_incls(&state, chapter_info, step.incl_opt);
        }

        Ok(chapter_info)
    }
}

#[async_trait]
impl<'a> Execute<ListPinnedInfosByComicIds<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListPinnedInfosByComicIds<'a>,
    ) -> Result<HashMap<String, ChapterInfo>, Self::Error> {
        let state = self.state.lock().unwrap();

        let mut chapter_infos = HashMap::new();

        for comic_id in step.comic_ids {
            let chapter_info = state
                .chapters
                .iter()
                .find(|chapter_info| chapter_info.comic_id == *comic_id && chapter_info.is_pinned)
                .cloned();

            let Some(chapter_info) = chapter_info else {
                continue;
            };

            chapter_infos.insert(comic_id.clone(), chapter_info);
        }

        Ok(chapter_infos)
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

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
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoById<'a>,
    ) -> Result<ChapterInfo, Self::Error> {
        get_chapter_by_id(&context.state, step.id, step.incl_opt)
    }
}

#[async_trait]
impl<'a> Advance<GetInfoByIdExcluded<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoByIdExcluded<'a>,
    ) -> Result<ChapterInfo, Self::Error> {
        get_chapter_by_id(&context.state, step.id, step.incl_opt)
    }
}

// #[async_trait]
// impl<'a> Advance<ListInfosByComicIdExcluded<'a>, MockContext> for MockTransactional {
//     ...
// }

#[async_trait]
impl<'a> Advance<ListAllInfosByComicIdExcluded<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

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
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &FindPinnedInfoByComicId<'a>,
    ) -> Result<Option<ChapterInfo>, Self::Error> {
        let mut chapter_info = context
            .state
            .chapters
            .iter()
            .find(|chapter_info| chapter_info.comic_id == step.comic_id && chapter_info.is_pinned)
            .cloned();

        if let Some(chapter_info) = &mut chapter_info {
            apply_chapter_incls(&context.state, chapter_info, step.incl_opt);
        }

        Ok(chapter_info)
    }
}

#[async_trait]
impl<'a> Advance<UpdateInfo<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

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
    type Error = RegularError;

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
    type Error = RegularError;

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
    type Error = RegularError;

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
    type Error = RegularError;

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
    type Error = RegularError;

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
