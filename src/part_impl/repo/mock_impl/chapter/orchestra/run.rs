use poprako_orchestra::Run;
use tracing::instrument;

use crate::model::read::proj::chapter::ChapterInfo;
use crate::part::repo::oper::chapter::{
    CompleteChapterRawProvide, FindPinnedChapterInfo, GetChapterInfo,
    ListChapterInfos, ListPinnedChapterInfos, StartChapterStage,
};
use crate::part_impl::repo::mock_impl::chapter::get_chapter_by_id;
use crate::part_impl::repo::mock_impl::chapter::orchestra::{
    find_pinned_chapter_info, list_chapter_infos, list_pinned_chapter_infos,
};
use crate::part_impl::repo::mock_impl::{Mock, now};
use crate::result::{BaseError, BaseRest, accept};
use crate::value::chapter::{Stage, StagePhase};

impl<'a> Run<ListChapterInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListChapterInfos<'a>,
    ) -> BaseRest<Vec<ChapterInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_chapter_infos(&state, oper.spec))
    }
}

impl<'a, 'b> Run<GetChapterInfo<'a, 'b>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &GetChapterInfo<'a, 'b>,
    ) -> BaseRest<ChapterInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        get_chapter_by_id(&state, oper.id, oper.incls)
    }
}

impl<'a, 'b> Run<FindPinnedChapterInfo<'a, 'b>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &FindPinnedChapterInfo<'a, 'b>,
    ) -> BaseRest<Option<ChapterInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(find_pinned_chapter_info(&state, oper.comic_id, oper.incls))
    }
}

impl<'a> Run<ListPinnedChapterInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListPinnedChapterInfos<'a>,
    ) -> BaseRest<Vec<ChapterInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_pinned_chapter_infos(&state, oper.comic_ids))
    }
}

impl<'a> Run<StartChapterStage<'a>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &StartChapterStage<'a>) -> BaseRest<bool> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let mut state = self.state.lock().unwrap();

        let Some(chapter_info) = state
            .chapters
            .iter_mut()
            .find(|chapter_info| chapter_info.id == oper.id)
        else {
            return accept(false);
        };

        if chapter_info
            .stages
            .has_phase(Stage::Publish, StagePhase::Completed)
        {
            return accept(false);
        }

        if !chapter_info
            .stages
            .has_phase(oper.stage, StagePhase::Pending)
        {
            return accept(false);
        }

        chapter_info.stages = chapter_info
            .stages
            .try_set_phase(oper.stage, StagePhase::Active)?;

        chapter_info.updated_at = now();

        accept(true)
    }
}

impl<'a> Run<CompleteChapterRawProvide<'a>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &CompleteChapterRawProvide<'a>,
    ) -> BaseRest<bool> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let mut state = self.state.lock().unwrap();

        let Some(chapter_index) = state
            .chapters
            .iter()
            .position(|chapter_info| chapter_info.id == oper.id)
        else {
            return accept(false);
        };

        if !state.chapters[chapter_index]
            .stages
            .has_phase(Stage::RawProvide, StagePhase::Pending)
        {
            return accept(false);
        }

        let page_count = state
            .pages
            .iter()
            .filter(|page_info| page_info.chapter_id == oper.id)
            .count();

        let all_pages_uploaded = page_count > 0
            && state.pages.iter().all(|page_info| {
                //
                page_info.chapter_id != oper.id
                    || page_info.is_image_uploaded.unwrap_or(false)
            });

        if !all_pages_uploaded {
            return accept(false);
        }

        let chapter_info = &mut state.chapters[chapter_index];

        chapter_info.stages = chapter_info
            .stages
            .try_set_phase(Stage::RawProvide, StagePhase::Completed)?;

        chapter_info.updated_at = now();

        accept(true)
    }
}
