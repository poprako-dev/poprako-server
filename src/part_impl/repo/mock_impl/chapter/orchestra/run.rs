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
use crate::result::{BaseError, BaseRest};
use crate::value::chapter::stage::{Stage, StagePhase};

impl<'a> Run<ListChapterInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ListChapterInfos<'a>,
    ) -> BaseRest<Vec<ChapterInfo>> {
        //
        let state = self.state.lock().unwrap();

        accept!(list_chapter_infos(&state, oper.spec))
    }
}

impl<'a, 'b> Run<GetChapterInfo<'a, 'b>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &GetChapterInfo<'a, 'b>,
    ) -> BaseRest<ChapterInfo> {
        //
        let state = self.state.lock().unwrap();

        get_chapter_by_id(&state, oper.id, oper.incls)
    }
}

impl<'a, 'b> Run<FindPinnedChapterInfo<'a, 'b>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &FindPinnedChapterInfo<'a, 'b>,
    ) -> BaseRest<Option<ChapterInfo>> {
        //
        let state = self.state.lock().unwrap();

        accept!(find_pinned_chapter_info(&state, oper.comic_id, oper.incls))
    }
}

impl<'a> Run<ListPinnedChapterInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ListPinnedChapterInfos<'a>,
    ) -> BaseRest<Vec<ChapterInfo>> {
        //
        let state = self.state.lock().unwrap();

        accept!(list_pinned_chapter_infos(&state, oper.comic_ids))
    }
}

impl<'a> Run<StartChapterStage<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &StartChapterStage<'a>) -> BaseRest<bool> {
        //
        {
            //
            // The state guard is released as soon as the transition completes.
            let mut state = self.state.lock().unwrap();

            if state.deleted_chapter_ids.contains(oper.id) {
                return accept!(false);
            }

            let Some(chapter_info) = state
                .chapters
                .iter_mut()
                .find(|chapter_info| chapter_info.id == oper.id)
            else {
                return accept!(false);
            };

            if chapter_info
                .stages
                .has_phase(Stage::Publish, StagePhase::Completed)
            {
                return accept!(false);
            }

            if !chapter_info
                .stages
                .has_phase(oper.stage, StagePhase::Pending)
            {
                return accept!(false);
            }

            chapter_info.stages = chapter_info
                .stages
                .try_set_phase(oper.stage, StagePhase::Active)?;

            state
                .chapters
                .iter_mut()
                .find(|chapter_info| chapter_info.id == oper.id)
                .unwrap()
                .updated_at = now();
        }

        accept!(true)
    }
}

impl<'a> Run<CompleteChapterRawProvide<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &CompleteChapterRawProvide<'a>,
    ) -> BaseRest<bool> {
        //
        {
            //
            // The state guard is released as soon as the transition completes.
            let mut state = self.state.lock().unwrap();

            if state.deleted_chapter_ids.contains(oper.id) {
                return accept!(false);
            }

            let Some(chapter_index) = state
                .chapters
                .iter()
                .position(|chapter_info| chapter_info.id == oper.id)
            else {
                return accept!(false);
            };

            if !state
                .chapters
                .get(chapter_index)
                .unwrap()
                .stages
                .has_phase(Stage::RawProvide, StagePhase::Pending)
            {
                return accept!(false);
            }

            if state.chapters.get(chapter_index).unwrap().page_source_count == 0
            {
                return accept!(false);
            }

            let chapter_info =
                &mut (*state.chapters.get_mut(chapter_index).unwrap());

            chapter_info.stages = chapter_info
                .stages
                .try_set_phase(Stage::RawProvide, StagePhase::Completed)?;

            state.chapters.get_mut(chapter_index).unwrap().updated_at = now();
        }

        accept!(true)
    }
}
