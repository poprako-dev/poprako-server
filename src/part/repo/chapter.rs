//! Repository traits for the chapter domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::Execute;
use crate::part::repo::step::chapter::{
    Create, Delete, FindPinnedByComicId, GetInfoById, GetInfoExcluded, ListByComicId,
    ListByComicIdExcluded, UnpinOthers, UpdateInfo, UpdateStage,
};
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional chapter repository.
pub trait ChapterRepo<C>:
    DeriveTransactional
    + for<'a> Execute<GetInfoById<'a>, Error = RootError>
    + for<'a> Execute<ListByComicId<'a>, Error = RootError>
    + for<'a> Execute<FindPinnedByComicId<'a>, Error = RootError>
where
    Self::Transactional: ChapterRepoTransactional<C>,
{
}

/// Transactional chapter repository.
pub trait ChapterRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoById<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<ListByComicIdExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<FindPinnedByComicId<'a>, C, Error = RootError>
    + for<'a> Advance<UpdateInfo<'a>, C, Error = RootError>
    + for<'a> Advance<UpdateStage<'a>, C, Error = RootError>
    + for<'a> Advance<UnpinOthers<'a>, C, Error = RootError>
    + for<'a> Advance<Delete<'a>, C, Error = RootError>
{
}
