use std::collections::HashMap;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::chapter::ChapterInfo;
use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCounters, CompleteChapterRawProvide, CreateChapter,
    DeleteChapter, FindPinnedChapterInfo, GetChapterInfo,
    GetChapterInfoExcluded, ListChapterInfos, ListChapterInfosExcluded,
    ListPinnedChapterInfos, SetChapterPageCounters, StartChapterStage,
    UnpinOtherChapters, UpdateChapter, UpdateChapterStage,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::chapter::{
    adjust_unit_counters, complete_raw_provide, create, delete,
    find_pinned_info_by_comic_id, get_info_by_id, get_info_excluded,
    list_infos, list_infos_excluded, list_pinned_infos_by_comic_ids,
    set_page_counters, start_stage, unpin_others, update_info, update_stage,
};
use crate::part_impl::shared::RdbContext;
use crate::result::{BaseError, BaseResult};

impl<'a, 'b> Run<GetChapterInfo<'a, 'b>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetChapterInfo<'a, 'b>,
    ) -> BaseResult<ChapterInfo> {
        submit_query!(self.core, get_info_by_id, oper.id, oper.incls)
    }
}

impl<'a> Run<ListChapterInfos<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListChapterInfos<'a>,
    ) -> BaseResult<Vec<ChapterInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl<'a, 'b> Run<FindPinnedChapterInfo<'a, 'b>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &FindPinnedChapterInfo<'a, 'b>,
    ) -> BaseResult<Option<ChapterInfo>> {
        submit_query!(
            self.core,
            find_pinned_info_by_comic_id,
            oper.comic_id,
            oper.incls
        )
    }
}

impl<'a> Run<ListPinnedChapterInfos<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListPinnedChapterInfos<'a>,
    ) -> BaseResult<HashMap<String, ChapterInfo>> {
        submit_query!(self.core, list_pinned_infos_by_comic_ids, oper.comic_ids)
    }
}

impl<'a> Run<StartChapterStage<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &StartChapterStage<'a>) -> BaseResult<bool> {
        submit_query!(self.core, start_stage, oper.id, oper.stage)
    }
}

impl<'a> Run<CompleteChapterRawProvide<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &CompleteChapterRawProvide<'a>,
    ) -> BaseResult<bool> {
        submit_query!(self.core, complete_raw_provide, oper.id)
    }
}

impl<'a, 'b> Step<GetChapterInfo<'a, 'b>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetChapterInfo<'a, 'b>,
    ) -> BaseResult<ChapterInfo> {
        get_info_by_id(context.conn(), oper.id, oper.incls).await
    }
}

impl<'a, 'b> Step<GetChapterInfoExcluded<'a, 'b>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetChapterInfoExcluded<'a, 'b>,
    ) -> BaseResult<ChapterInfo> {
        get_info_excluded(context.conn(), oper.id, oper.incls).await
    }
}

impl<'a> Step<ListChapterInfosExcluded<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListChapterInfosExcluded<'a>,
    ) -> BaseResult<Vec<ChapterInfo>> {
        list_infos_excluded(context.conn(), oper.comic_id).await
    }
}

impl<'a, 'b> Step<FindPinnedChapterInfo<'a, 'b>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &FindPinnedChapterInfo<'a, 'b>,
    ) -> BaseResult<Option<ChapterInfo>> {
        find_pinned_info_by_comic_id(context.conn(), oper.comic_id, oper.incls)
            .await
    }
}

impl<'a> Step<CreateChapter<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateChapter<'a>,
    ) -> BaseResult<ChapterInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<'a> Step<UpdateChapter<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateChapter<'a>,
    ) -> BaseResult<()> {
        update_info(context.conn(), oper.update).await
    }
}

impl<'a> Step<UpdateChapterStage<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateChapterStage<'a>,
    ) -> BaseResult<()> {
        update_stage(context.conn(), oper.update).await
    }
}

impl<'a> Step<SetChapterPageCounters<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &SetChapterPageCounters<'a>,
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

impl<'a> Step<AdjustChapterUnitCounters<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &AdjustChapterUnitCounters<'a>,
    ) -> BaseResult<()> {
        adjust_unit_counters(context.conn(), oper.id, &oper.delta).await
    }
}

impl<'a> Step<UnpinOtherChapters<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UnpinOtherChapters<'a>,
    ) -> BaseResult<()> {
        unpin_others(context.conn(), oper.comic_id, oper.excluded_id).await
    }
}

impl<'a> Step<DeleteChapter<'a>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteChapter<'a>,
    ) -> BaseResult<()> {
        delete(context.conn(), oper.id).await
    }
}
