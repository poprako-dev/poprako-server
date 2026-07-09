//! Repository traits for the chapter domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::chapter::{
    AdjustUnitCounters, Create, Delete, FindPinnedInfoByComicId, GetInfoById,
    GetInfoByIdExcluded, ListAllInfosByComicIdExcluded, ListInfos,
    ListPinnedInfosByComicIds, SetPageCounters, UnpinOthers, UpdateInfo,
    UpdateStage,
};
use crate::part::shared::execute::Execute;
use crate::result::RegularError;
use crate::util::DeriveTransactional;

/// Non-transactional chapter repository.
pub trait ChapterRepo<C>:
    DeriveTransactional
    + for<'a> Execute<GetInfoById<'a>, Error = RegularError>
    + for<'a> Execute<ListInfos<'a>, Error = RegularError>
    // + for<'a> Execute<ListInfosByComicId<'a>, Error = RegularError>
    + for<'a> Execute<FindPinnedInfoByComicId<'a>, Error = RegularError>
    + for<'a> Execute<ListPinnedInfosByComicIds<'a>, Error = RegularError>
where
    Self::Transactional: ChapterRepoTransactional<C>,
{
}

/// Transactional chapter repository.
pub trait ChapterRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RegularError>
    + for<'a> Advance<GetInfoById<'a>, C, Error = RegularError>
    + for<'a> Advance<GetInfoByIdExcluded<'a>, C, Error = RegularError>
    // + for<'a> Advance<ListInfosByComicIdExcluded<'a>, C, Error = RegularError>
    + for<'a> Advance<ListAllInfosByComicIdExcluded<'a>, C, Error = RegularError>
    + for<'a> Advance<FindPinnedInfoByComicId<'a>, C, Error = RegularError>
    + for<'a> Advance<UpdateInfo<'a>, C, Error = RegularError>
    + for<'a> Advance<UpdateStage<'a>, C, Error = RegularError>
    + for<'a> Advance<SetPageCounters<'a>, C, Error = RegularError>
    + for<'a> Advance<AdjustUnitCounters<'a>, C, Error = RegularError>
    + for<'a> Advance<UnpinOthers<'a>, C, Error = RegularError>
    + for<'a> Advance<Delete<'a>, C, Error = RegularError>
{
}
