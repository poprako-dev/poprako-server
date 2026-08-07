use poprako_orchestra::Step;
use tracing::instrument;

use crate::model::read::proj::chapter::ChapterInfo;
use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCounters, CompleteChapterRawProvide, CreateChapter,
    DeleteChapter, FindPinnedChapterInfo, GetChapterInfo,
    GetChapterInfoExcluded, ListChapterInfosExcluded, LockChapters,
    ResetChapterRawProvide, SetChapterPageCounters, UnpinOtherChapters,
    UpdateChapter, UpdateChapterStage,
};
use crate::part_impl::repo::mock_impl::chapter::orchestra::find_pinned_chapter_info;
use crate::part_impl::repo::mock_impl::chapter::{
    apply_chapter_incls, create_chapter, get_chapter_by_id, list_infos,
};
use crate::part_impl::repo::mock_impl::{Mock, MockContext, expected, now};
use crate::result::{BaseError, BaseRest, accept};
use crate::value::chapter::{ChapterInclOpt, Stage, StagePhase};

impl<'a> Step<CompleteChapterRawProvide<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CompleteChapterRawProvide<'a>,
    ) -> BaseRest<bool> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let Some(chapter_index) = context
            .state
            .chapters
            .iter()
            .position(|chapter_info| chapter_info.id == oper.id)
        else {
            return accept(false);
        };

        if !context.state.chapters[chapter_index]
            .stages
            .has_phase(Stage::RawProvide, StagePhase::Pending)
        {
            return accept(false);
        }

        let page_count = context
            .state
            .pages
            .iter()
            .filter(|page_info| page_info.chapter_id == oper.id)
            .count();

        let all_pages_uploaded = page_count > 0
            && context.state.pages.iter().all(|page_info| {
                //
                page_info.chapter_id != oper.id
                    || page_info.is_image_uploaded.unwrap_or(false)
            });

        if !all_pages_uploaded {
            return accept(false);
        }

        let chapter_info = &mut context.state.chapters[chapter_index];

        chapter_info.stages = chapter_info
            .stages
            .try_set_phase(Stage::RawProvide, StagePhase::Completed)?;

        chapter_info.updated_at = now();

        accept(true)
    }
}

impl<'a> Step<ResetChapterRawProvide<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ResetChapterRawProvide<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let chapter_info = context
            .state
            .chapters
            .iter_mut()
            .find(|chapter_info| chapter_info.id == oper.id)
            .ok_or_else(|| expected("error-chapter-not-found"))?;

        chapter_info.stages = chapter_info
            .stages
            .try_set_phase(Stage::RawProvide, StagePhase::Pending)?;

        chapter_info.updated_at = now();

        accept(())
    }
}

impl<'a, 'b> Step<GetChapterInfo<'a, 'b>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetChapterInfo<'a, 'b>,
    ) -> BaseRest<ChapterInfo> {
        get_chapter_by_id(&context.state, oper.id, oper.incls)
    }
}

impl<'a, 'b> Step<GetChapterInfoExcluded<'a, 'b>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetChapterInfoExcluded<'a, 'b>,
    ) -> BaseRest<ChapterInfo> {
        get_chapter_by_id(&context.state, oper.id, oper.incls)
    }
}

impl<'a> Step<ListChapterInfosExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListChapterInfosExcluded<'a>,
    ) -> BaseRest<Vec<ChapterInfo>> {
        accept(list_infos(&context.state, oper.comic_id))
    }
}

impl<'a> Step<LockChapters<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        _: &mut MockContext,
        _: &LockChapters<'a>,
    ) -> BaseRest<()> {
        accept(())
    }
}

impl<'a, 'b> Step<FindPinnedChapterInfo<'a, 'b>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &FindPinnedChapterInfo<'a, 'b>,
    ) -> BaseRest<Option<ChapterInfo>> {
        //
        accept(find_pinned_chapter_info(
            &context.state,
            oper.comic_id,
            oper.incls,
        ))
    }
}

impl<'a> Step<CreateChapter<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateChapter<'a>,
    ) -> BaseRest<ChapterInfo> {
        create_chapter(&mut context.state, oper.entry)
    }
}

impl<'a> Step<UpdateChapter<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateChapter<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
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

        accept(())
    }
}

impl<'a> Step<UpdateChapterStage<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateChapterStage<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let chapter_info = context
            .state
            .chapters
            .iter_mut()
            .find(|chapter_info| chapter_info.id == oper.update.id)
            .ok_or_else(|| expected("error-chapter-not-found"))?;

        chapter_info.stages = oper.update.stages;

        chapter_info.updated_at = now();

        accept(())
    }
}

impl<'a> Step<SetChapterPageCounters<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &SetChapterPageCounters<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
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

        accept(())
    }
}

impl<'a> Step<AdjustChapterUnitCounters<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &AdjustChapterUnitCounters<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
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

        accept(())
    }
}

impl<'a> Step<UnpinOtherChapters<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UnpinOtherChapters<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        for chapter_info in &mut context.state.chapters {
            //
            if chapter_info.comic_id == oper.comic_id
                && chapter_info.id != oper.excluded_id
            {
                chapter_info.is_pinned = false;

                chapter_info.updated_at = now();
            }
        }

        accept(())
    }
}

impl<'a> Step<DeleteChapter<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteChapter<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
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

        accept(())
    }
}
