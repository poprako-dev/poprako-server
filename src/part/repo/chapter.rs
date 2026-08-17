use poprako_orchestra::drive;

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
/// Independent queries use [`poprako_orchestra::Run`]. Coordinated queries, mutations, and
/// pessimistic reads use [`poprako_orchestra::Step`] with the caller-coordinated context.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a, 'b> GetChapterInfo<'a, 'b>,
        for<'a> ListChapterInfos<'a>,
        for<'a, 'b> FindPinnedChapterInfo<'a, 'b>,
        for<'a> ListPinnedChapterInfos<'a>,
        for<'a> StartChapterStage<'a>,
        for<'a> CompleteChapterRawProvide<'a>,
    ),
    step(
        for<'a, 'b> GetChapterInfo<'a, 'b>,
        for<'a, 'b> GetChapterInfoExcluded<'a, 'b>,
        for<'a> ListChapterInfosExcluded<'a>,
        for<'a> LockChapters<'a>,
        for<'a, 'b> FindPinnedChapterInfo<'a, 'b>,
        for<'a> CreateChapter<'a>,
        for<'a> UpdateChapter<'a>,
        for<'a> UpdateChapterStage<'a>,
        for<'a> StartChapterStage<'a>,
        for<'a> CompleteChapterRawProvide<'a>,
        for<'a> ResetChapterRawProvide<'a>,
        for<'a> SetChapterPageCounters<'a>,
        for<'a> AdjustChapterUnitCounters<'a>,
        for<'a> UnpinOtherChapters<'a>,
        for<'a> DeleteChapter<'a>,
    ),
)]
pub trait ChapterRepo<C> {}
