//! Repository traits for the page domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::page::{
    CreateBatch, DeleteByChapterId, GetInfoById, GetInfoExcluded, ListAllInfosByChapter,
    ListInfosByChapter, MarkImageUploaded, ReserveImage, SetUnitCounters,
};
use crate::part::shared::execute::Execute;
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional page repository.
pub trait PageRepo<C>:
    DeriveTransactional
    + for<'a> Execute<GetInfoById<'a>, Error = RootError>
    + for<'a> Execute<ListInfosByChapter<'a>, Error = RootError>
where
    Self::Transactional: PageRepoTransactional<C>,
{
}

/// Transactional page repository.
pub trait PageRepoTransactional<C>:
    for<'a> Advance<GetInfoById<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<ListInfosByChapter<'a>, C, Error = RootError>
    + for<'a> Advance<ListAllInfosByChapter<'a>, C, Error = RootError>
    + for<'a> Advance<CreateBatch<'a>, C, Error = RootError>
    + for<'a> Advance<ReserveImage<'a>, C, Error = RootError>
    + for<'a> Advance<MarkImageUploaded<'a>, C, Error = RootError>
    + for<'a> Advance<SetUnitCounters<'a>, C, Error = RootError>
    + for<'a> Advance<DeleteByChapterId<'a>, C, Error = RootError>
{
}
