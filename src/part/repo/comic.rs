//! Repository traits for the comic domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::Execute;
use crate::part::repo::step::comic::{
    Create, Delete, GetInfoById, GetInfoExcluded, ListByWorksetId, ListByWorksetIdExcluded,
    MarkCompleted, MarkCoverUploaded, ReserveCover, UpdateInfo,
};
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional comic repository.
pub trait ComicRepo<C>:
    DeriveTransactional
    + for<'a> Execute<GetInfoById<'a>, Error = RootError>
    + for<'a> Execute<ListByWorksetId<'a>, Error = RootError>
    + for<'a> Execute<UpdateInfo<'a>, Error = RootError>
    + for<'a> Execute<MarkCoverUploaded<'a>, Error = RootError>
where
    Self::Transactional: ComicRepoTransactional<C>,
{
}

/// Transactional comic repository.
pub trait ComicRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoById<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<ListByWorksetIdExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<ReserveCover<'a>, C, Error = RootError>
    + for<'a> Advance<MarkCoverUploaded<'a>, C, Error = RootError>
    + for<'a> Advance<Delete<'a>, C, Error = RootError>
    + for<'a> Advance<MarkCompleted<'a>, C, Error = RootError>
{
}
