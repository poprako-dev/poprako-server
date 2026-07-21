use std::collections::HashMap;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::chapter::ChapterInfo;
use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCounters, CompleteChapterRawProvide, CreateChapter,
    DeleteChapter, FindPinnedChapterInfo, GetChapterInfo,
    GetChapterInfoExcluded, ListChapterInfos, ListChapterInfosExcluded,
    ListPinnedChapterInfos, ResetChapterRawProvide, SetChapterPageCounters,
    StartChapterStage, UnpinOtherChapters, UpdateChapter, UpdateChapterStage,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::chapter::step_impl::{
    adjust_unit_counters, complete_raw_provide, create, delete,
    find_pinned_info_by_comic_id, get_info_by_id, get_info_excluded,
    list_infos, list_infos_excluded, list_pinned_infos_by_comic_ids,
    reset_raw_provide, set_page_counters, start_stage, unpin_others,
    update_info, update_stage,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult};

impl Run<GetChapterInfo<'_, '_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetChapterInfo<'_, '_>,
    ) -> BaseResult<ChapterInfo> {
        submit_query!(self.core, get_info_by_id, oper.id, oper.incls)
    }
}

impl Run<ListChapterInfos<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListChapterInfos<'_>,
    ) -> BaseResult<Vec<ChapterInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl Run<FindPinnedChapterInfo<'_, '_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &FindPinnedChapterInfo<'_, '_>,
    ) -> BaseResult<Option<ChapterInfo>> {
        submit_query!(
            self.core,
            find_pinned_info_by_comic_id,
            oper.comic_id,
            oper.incls
        )
    }
}

impl Run<ListPinnedChapterInfos<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListPinnedChapterInfos<'_>,
    ) -> BaseResult<HashMap<String, ChapterInfo>> {
        submit_query!(self.core, list_pinned_infos_by_comic_ids, oper.comic_ids)
    }
}

impl Run<StartChapterStage<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &StartChapterStage<'_>) -> BaseResult<bool> {
        submit_query!(self.core, start_stage, oper.id, oper.stage)
    }
}

impl Run<CompleteChapterRawProvide<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &CompleteChapterRawProvide<'_>,
    ) -> BaseResult<bool> {
        submit_query!(self.core, complete_raw_provide, oper.id)
    }
}

impl Step<CompleteChapterRawProvide<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CompleteChapterRawProvide<'_>,
    ) -> BaseResult<bool> {
        complete_raw_provide(context.conn(), oper.id).await
    }
}

impl Step<ResetChapterRawProvide<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ResetChapterRawProvide<'_>,
    ) -> BaseResult<()> {
        reset_raw_provide(context.conn(), oper.id).await
    }
}

impl Step<GetChapterInfo<'_, '_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetChapterInfo<'_, '_>,
    ) -> BaseResult<ChapterInfo> {
        get_info_by_id(context.conn(), oper.id, oper.incls).await
    }
}

impl Step<GetChapterInfoExcluded<'_, '_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetChapterInfoExcluded<'_, '_>,
    ) -> BaseResult<ChapterInfo> {
        get_info_excluded(context.conn(), oper.id, oper.incls).await
    }
}

impl Step<ListChapterInfosExcluded<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListChapterInfosExcluded<'_>,
    ) -> BaseResult<Vec<ChapterInfo>> {
        list_infos_excluded(context.conn(), oper.comic_id).await
    }
}

impl Step<FindPinnedChapterInfo<'_, '_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &FindPinnedChapterInfo<'_, '_>,
    ) -> BaseResult<Option<ChapterInfo>> {
        find_pinned_info_by_comic_id(context.conn(), oper.comic_id, oper.incls)
            .await
    }
}

impl Step<CreateChapter<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateChapter<'_>,
    ) -> BaseResult<ChapterInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl Step<UpdateChapter<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateChapter<'_>,
    ) -> BaseResult<()> {
        update_info(context.conn(), oper.update).await
    }
}

impl Step<UpdateChapterStage<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateChapterStage<'_>,
    ) -> BaseResult<()> {
        update_stage(context.conn(), oper.update).await
    }
}

impl Step<SetChapterPageCounters<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &SetChapterPageCounters<'_>,
    ) -> BaseResult<()> {
        set_page_counters(
            context.conn(),
            oper.id,
            oper.page_count,
            oper.total_unit_count,
            oper.translated_unit_count,
            oper.proofread_unit_count,
        )
        .await
    }
}

impl Step<AdjustChapterUnitCounters<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &AdjustChapterUnitCounters<'_>,
    ) -> BaseResult<()> {
        adjust_unit_counters(context.conn(), oper.id, &oper.delta).await
    }
}

impl Step<UnpinOtherChapters<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UnpinOtherChapters<'_>,
    ) -> BaseResult<()> {
        unpin_others(context.conn(), oper.comic_id, oper.excluded_id).await
    }
}

impl Step<DeleteChapter<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteChapter<'_>,
    ) -> BaseResult<()> {
        delete(context.conn(), oper.id).await
    }
}
