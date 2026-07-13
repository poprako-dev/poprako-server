use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::chapter::{
    AdjustChapterUnitCounters, CreateChapter, DeleteChapter,
    FindPinnedChapterInfo, GetChapterInfo, GetChapterInfoExcluded,
    ListChapterInfos, ListChapterInfosExcluded, ListPinnedChapterInfos,
    SetChapterPageCounters, UnpinOtherChapters, UpdateChapter,
    UpdateChapterStage,
};
use crate::result::RegularError;

/// Chapter repository operations.
///
/// Independent queries use [`Run`]. Coordinated queries, mutations, and
/// pessimistic reads use [`Step`] with the caller-coordinated context.
pub trait ChapterRepo<C>:
    for<'a, 'b> Run<GetChapterInfo<'a, 'b>, Error = RegularError>
    + for<'a> Run<ListChapterInfos<'a>, Error = RegularError>
    + for<'a, 'b> Run<FindPinnedChapterInfo<'a, 'b>, Error = RegularError>
    + for<'a> Run<ListPinnedChapterInfos<'a>, Error = RegularError>
    + for<'a, 'b> Step<GetChapterInfo<'a, 'b>, C, Error = RegularError>
    + for<'a, 'b> Step<GetChapterInfoExcluded<'a, 'b>, C, Error = RegularError>
    + for<'a> Step<ListChapterInfosExcluded<'a>, C, Error = RegularError>
    + for<'a, 'b> Step<FindPinnedChapterInfo<'a, 'b>, C, Error = RegularError>
    + for<'a> Step<CreateChapter<'a>, C, Error = RegularError>
    + for<'a> Step<UpdateChapter<'a>, C, Error = RegularError>
    + for<'a> Step<UpdateChapterStage<'a>, C, Error = RegularError>
    + for<'a> Step<SetChapterPageCounters<'a>, C, Error = RegularError>
    + for<'a> Step<AdjustChapterUnitCounters<'a>, C, Error = RegularError>
    + for<'a> Step<UnpinOtherChapters<'a>, C, Error = RegularError>
    + for<'a> Step<DeleteChapter<'a>, C, Error = RegularError>
{
}
