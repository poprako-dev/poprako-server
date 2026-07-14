use std::collections::HashMap;

use poprako_orchestra::{Run, Step};

use tracing::instrument;

use crate::model::chapter::{ChapterInfo, ChapterInfoListSpec};
use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCounters, CreateChapter, DeleteChapter,
    FindPinnedChapterInfo, GetChapterInfo, GetChapterInfoExcluded,
    ListChapterInfos, ListChapterInfosExcluded, ListPinnedChapterInfos,
    SetChapterPageCounters, UnpinOtherChapters, UpdateChapter,
    UpdateChapterStage,
};
use crate::part_impl::repo::mock_impl::chapter::{
    apply_chapter_incls, create_chapter, get_chapter_by_id, list_all_chapters,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{RegularError, RegularResult};
use crate::value::chapter::ChapterInclOpt;

fn list_chapter_infos(
    state: &MockState,
    spec: &ChapterInfoListSpec,
) -> Vec<ChapterInfo> {
    //
    let mut chapter_infos = list_all_chapters(state, &spec.comic_id);

    for chapter_info in &mut chapter_infos {
        apply_chapter_incls(state, chapter_info, &spec.incl_opt);
    }

    let offset = spec.offset as usize;

    let limit = spec.limit as usize;

    match offset >= chapter_infos.len() {
        //
        true => Vec::new(),

        false => {
            //
            let end = std::cmp::min(offset + limit, chapter_infos.len());

            chapter_infos[offset..end].to_vec()
        }
    }
}

fn find_pinned_chapter_info(
    state: &MockState,
    comic_id: &str,
    incls: &[ChapterInclOpt],
) -> Option<ChapterInfo> {
    //
    let mut chapter_info = state
        .chapters
        .iter()
        .find(|chapter_info| {
            chapter_info.comic_id == comic_id && chapter_info.is_pinned
        })
        .cloned();

    if let Some(chapter_info) = &mut chapter_info {
        apply_chapter_incls(state, chapter_info, incls);
    }

    chapter_info
}

fn list_pinned_chapter_infos(
    state: &MockState,
    comic_ids: &[String],
) -> HashMap<String, ChapterInfo> {
    //
    let mut chapter_infos = HashMap::new();

    for comic_id in comic_ids {
        //
        let chapter_info = state
            .chapters
            .iter()
            .find(|chapter_info| {
                chapter_info.comic_id == *comic_id && chapter_info.is_pinned
            })
            .cloned();

        let Some(chapter_info) = chapter_info else {
            continue;
        };

        chapter_infos.insert(comic_id.clone(), chapter_info);
    }

    chapter_infos
}

impl<'a> Run<ListChapterInfos<'a>> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListChapterInfos<'a>,
    ) -> RegularResult<Vec<ChapterInfo>> {
        //
        let state = self.state.lock().unwrap();

        Ok(list_chapter_infos(&state, oper.spec))
    }
}

impl<'a, 'b> Run<GetChapterInfo<'a, 'b>> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetChapterInfo<'a, 'b>,
    ) -> RegularResult<ChapterInfo> {
        //
        let state = self.state.lock().unwrap();

        get_chapter_by_id(&state, oper.id, oper.incls)
    }
}

impl<'a, 'b> Run<FindPinnedChapterInfo<'a, 'b>> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &FindPinnedChapterInfo<'a, 'b>,
    ) -> RegularResult<Option<ChapterInfo>> {
        //
        let state = self.state.lock().unwrap();

        Ok(find_pinned_chapter_info(&state, oper.comic_id, oper.incls))
    }
}

impl<'a> Run<ListPinnedChapterInfos<'a>> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListPinnedChapterInfos<'a>,
    ) -> RegularResult<HashMap<String, ChapterInfo>> {
        //
        let state = self.state.lock().unwrap();

        Ok(list_pinned_chapter_infos(&state, oper.comic_ids))
    }
}

impl<'a, 'b> Step<GetChapterInfo<'a, 'b>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetChapterInfo<'a, 'b>,
    ) -> RegularResult<ChapterInfo> {
        get_chapter_by_id(&context.state, oper.id, oper.incls)
    }
}

impl<'a, 'b> Step<GetChapterInfoExcluded<'a, 'b>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetChapterInfoExcluded<'a, 'b>,
    ) -> RegularResult<ChapterInfo> {
        get_chapter_by_id(&context.state, oper.id, oper.incls)
    }
}

impl<'a> Step<ListChapterInfosExcluded<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListChapterInfosExcluded<'a>,
    ) -> RegularResult<Vec<ChapterInfo>> {
        Ok(list_all_chapters(&context.state, oper.comic_id))
    }
}

impl<'a, 'b> Step<FindPinnedChapterInfo<'a, 'b>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &FindPinnedChapterInfo<'a, 'b>,
    ) -> RegularResult<Option<ChapterInfo>> {
        Ok(find_pinned_chapter_info(
            &context.state,
            oper.comic_id,
            oper.incls,
        ))
    }
}

impl<'a> Step<CreateChapter<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateChapter<'a>,
    ) -> RegularResult<ChapterInfo> {
        create_chapter(&mut context.state, oper.entry)
    }
}

impl<'a> Step<UpdateChapter<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateChapter<'a>,
    ) -> RegularResult<()> {
        //
        let chapter_info = context
            .state
            .chapters
            .iter_mut()
            .find(|chapter_info| chapter_info.id == oper.update.id)
            .ok_or_else(|| expected("error-chapter-not-found"))?;

        if let Some(subtitle) = &oper.update.subtitle {
            chapter_info.subtitle = subtitle.clone();
        }

        if let Some(is_pinned) = oper.update.pin {
            chapter_info.is_pinned = is_pinned;
        }

        chapter_info.updated_at = now();

        Ok(())
    }
}

impl<'a> Step<UpdateChapterStage<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateChapterStage<'a>,
    ) -> RegularResult<()> {
        //
        let chapter_info = context
            .state
            .chapters
            .iter_mut()
            .find(|chapter_info| chapter_info.id == oper.update.id)
            .ok_or_else(|| expected("error-chapter-not-found"))?;

        chapter_info.stages = oper.update.stages;

        chapter_info.updated_at = now();

        Ok(())
    }
}

impl<'a> Step<SetChapterPageCounters<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &SetChapterPageCounters<'a>,
    ) -> RegularResult<()> {
        //
        let chapter_info = context
            .state
            .chapters
            .iter_mut()
            .find(|chapter_info| chapter_info.id == oper.id)
            .ok_or_else(|| expected("error-chapter-not-found"))?;

        chapter_info.page_count = oper.page_count;

        chapter_info.total_unit_count = oper.total_unit_count;

        chapter_info.translated_unit_count = oper.translated_unit_count;

        chapter_info.proofread_unit_count = oper.proofread_unit_count;

        chapter_info.updated_at = now();

        Ok(())
    }
}

impl<'a> Step<AdjustChapterUnitCounters<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &AdjustChapterUnitCounters<'a>,
    ) -> RegularResult<()> {
        //
        let chapter_info = context
            .state
            .chapters
            .iter_mut()
            .find(|chapter_info| chapter_info.id == oper.id)
            .ok_or_else(|| expected("error-chapter-not-found"))?;

        chapter_info.total_unit_count += oper.delta.total_unit_count;

        chapter_info.translated_unit_count += oper.delta.translated_unit_count;

        chapter_info.proofread_unit_count += oper.delta.proofread_unit_count;

        chapter_info.updated_at = now();

        Ok(())
    }
}

impl<'a> Step<UnpinOtherChapters<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UnpinOtherChapters<'a>,
    ) -> RegularResult<()> {
        //
        for chapter_info in &mut context.state.chapters {
            if chapter_info.comic_id == oper.comic_id
                && chapter_info.id != oper.excluded_id
            {
                chapter_info.is_pinned = false;

                chapter_info.updated_at = now();
            }
        }

        Ok(())
    }
}

impl<'a> Step<DeleteChapter<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteChapter<'a>,
    ) -> RegularResult<()> {
        //
        let position = context
            .state
            .chapters
            .iter()
            .position(|chapter_info| chapter_info.id == oper.id)
            .ok_or_else(|| expected("error-chapter-not-found"))?;

        context.state.chapters.remove(position);

        context
            .state
            .pages
            .retain(|page_info| page_info.chapter_id != oper.id);

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
            .retain(|assignment_info| assignment_info.chapter_id != oper.id);

        Ok(())
    }
}
