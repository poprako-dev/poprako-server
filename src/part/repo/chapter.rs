//! Repository traits for the chapter domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::chapter::{
    Create, Delete, FindPinnedInfoByComicId, GetInfoById, GetInfoExcluded,
    ListAllInfosByComicIdExcluded, ListInfosByComicId, ListInfosByComicIdExcluded, UnpinOthers,
    UpdateInfo, UpdateStage,
};
use crate::part::shared::execute::Execute;
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional chapter repository.
pub trait ChapterRepo<C>:
    DeriveTransactional
    + for<'a> Execute<GetInfoById<'a>, Error = RootError>
    + for<'a> Execute<ListInfosByComicId<'a>, Error = RootError>
    + for<'a> Execute<FindPinnedInfoByComicId<'a>, Error = RootError>
where
    Self::Transactional: ChapterRepoTransactional<C>,
{
}

/// Transactional chapter repository.
pub trait ChapterRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoById<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<ListInfosByComicIdExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<ListAllInfosByComicIdExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<FindPinnedInfoByComicId<'a>, C, Error = RootError>
    + for<'a> Advance<UpdateInfo<'a>, C, Error = RootError>
    + for<'a> Advance<UpdateStage<'a>, C, Error = RootError>
    + for<'a> Advance<UnpinOthers<'a>, C, Error = RootError>
    + for<'a> Advance<Delete<'a>, C, Error = RootError>
{
}
