use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCounters, CompleteChapterRawProvide, CreateChapter,
    DeleteChapter, FindPinnedChapterInfo, GetChapterInfo,
    GetChapterInfoExcluded, ListChapterInfos, ListChapterInfosExcluded,
    ListPinnedChapterInfos, LockChapters, ResetChapterRawProvide,
    SetChapterPageCounters, StartChapterStage, UnpinOtherChapters,
    UpdateChapter, UpdateChapterStage,
};
use crate::result::BaseError;

/// Chapter repository operations.
///
/// Independent queries use [`Run`]. Coordinated queries, mutations, and
/// pessimistic reads use [`Step`] with the caller-coordinated context.
pub trait ChapterRepo<C>:
    for<'a, 'b> Run<GetChapterInfo<'a, 'b>, Error = BaseError>
    + for<'a> Run<ListChapterInfos<'a>, Error = BaseError>
    + for<'a, 'b> Run<FindPinnedChapterInfo<'a, 'b>, Error = BaseError>
    + for<'a> Run<ListPinnedChapterInfos<'a>, Error = BaseError>
    + for<'a> Run<StartChapterStage<'a>, Error = BaseError>
    + for<'a> Run<CompleteChapterRawProvide<'a>, Error = BaseError>
    + for<'a, 'b> Step<GetChapterInfo<'a, 'b>, C, Error = BaseError>
    + for<'a, 'b> Step<GetChapterInfoExcluded<'a, 'b>, C, Error = BaseError>
    + for<'a> Step<ListChapterInfosExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<LockChapters<'a>, C, Error = BaseError>
    + for<'a, 'b> Step<FindPinnedChapterInfo<'a, 'b>, C, Error = BaseError>
    + for<'a> Step<CreateChapter<'a>, C, Error = BaseError>
    + for<'a> Step<UpdateChapter<'a>, C, Error = BaseError>
    + for<'a> Step<UpdateChapterStage<'a>, C, Error = BaseError>
    + for<'a> Step<CompleteChapterRawProvide<'a>, C, Error = BaseError>
    + for<'a> Step<ResetChapterRawProvide<'a>, C, Error = BaseError>
    + for<'a> Step<SetChapterPageCounters<'a>, C, Error = BaseError>
    + for<'a> Step<AdjustChapterUnitCounters<'a>, C, Error = BaseError>
    + for<'a> Step<UnpinOtherChapters<'a>, C, Error = BaseError>
    + for<'a> Step<DeleteChapter<'a>, C, Error = BaseError>
{
}

impl<T, C> ChapterRepo<C> for T where
    T: for<'a, 'b> Run<GetChapterInfo<'a, 'b>, Error = BaseError>
        + for<'a> Run<ListChapterInfos<'a>, Error = BaseError>
        + for<'a, 'b> Run<FindPinnedChapterInfo<'a, 'b>, Error = BaseError>
        + for<'a> Run<ListPinnedChapterInfos<'a>, Error = BaseError>
        + for<'a> Run<StartChapterStage<'a>, Error = BaseError>
        + for<'a> Run<CompleteChapterRawProvide<'a>, Error = BaseError>
        + for<'a, 'b> Step<GetChapterInfo<'a, 'b>, C, Error = BaseError>
        + for<'a, 'b> Step<GetChapterInfoExcluded<'a, 'b>, C, Error = BaseError>
        + for<'a> Step<ListChapterInfosExcluded<'a>, C, Error = BaseError>
        + for<'a> Step<LockChapters<'a>, C, Error = BaseError>
        + for<'a, 'b> Step<FindPinnedChapterInfo<'a, 'b>, C, Error = BaseError>
        + for<'a> Step<CreateChapter<'a>, C, Error = BaseError>
        + for<'a> Step<UpdateChapter<'a>, C, Error = BaseError>
        + for<'a> Step<UpdateChapterStage<'a>, C, Error = BaseError>
        + for<'a> Step<CompleteChapterRawProvide<'a>, C, Error = BaseError>
        + for<'a> Step<ResetChapterRawProvide<'a>, C, Error = BaseError>
        + for<'a> Step<SetChapterPageCounters<'a>, C, Error = BaseError>
        + for<'a> Step<AdjustChapterUnitCounters<'a>, C, Error = BaseError>
        + for<'a> Step<UnpinOtherChapters<'a>, C, Error = BaseError>
        + for<'a> Step<DeleteChapter<'a>, C, Error = BaseError>
{
}
