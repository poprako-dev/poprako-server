//! Repository traits for the comic domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::comic::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrChapterNextIndex,
    ListInfos, ListInfosExcluded, MarkCompleted, MarkCoverUploaded,
    ReserveCover, TouchLastActive, UpdateChapterCount, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::result::RegularError;
use crate::util::DeriveTransactional;

/// Non-transactional comic repository.
pub trait ComicRepo<C>:
    DeriveTransactional
    + for<'a> Execute<GetInfoById<'a>, Error = RegularError>
    + for<'a> Execute<ListInfos<'a>, Error = RegularError>
    + for<'a> Execute<UpdateInfo<'a>, Error = RegularError>
    + for<'a> Execute<MarkCoverUploaded<'a>, Error = RegularError>
where
    Self::Transactional: ComicRepoTransactional<C>,
{
}

/// Transactional comic repository.
pub trait ComicRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RegularError>
    + for<'a> Advance<GetInfoById<'a>, C, Error = RegularError>
    + for<'a> Advance<GetInfoExcluded<'a>, C, Error = RegularError>
    + for<'a> Advance<ListInfosExcluded<'a>, C, Error = RegularError>
    + for<'a> Advance<ReserveCover<'a>, C, Error = RegularError>
    + for<'a> Advance<MarkCoverUploaded<'a>, C, Error = RegularError>
    + for<'a> Advance<Delete<'a>, C, Error = RegularError>
    + for<'a> Advance<MarkCompleted<'a>, C, Error = RegularError>
    + for<'a> Advance<IncrChapterNextIndex<'a>, C, Error = RegularError>
    + for<'a> Advance<UpdateChapterCount<'a>, C, Error = RegularError>
    + for<'a> Advance<TouchLastActive<'a>, C, Error = RegularError>
{
}
