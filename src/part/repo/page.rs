//! Repository traits for the page domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::page::{
    CreateBatch, DeleteByChapterId, GetInfoById, GetInfoExcluded, ListAllInfosByChapterId,
    ListInfosByChapterId, MarkImageUploaded, ReserveImage, SetUnitCounters,
};
use crate::part::shared::execute::Execute;
use crate::result::RegularError;
use crate::util::DeriveTransactional;

/// Non-transactional page repository.
pub trait PageRepo<C>:
    DeriveTransactional
    + for<'a> Execute<GetInfoById<'a>, Error = RegularError>
    + for<'a> Execute<ListInfosByChapterId<'a>, Error = RegularError>
where
    Self::Transactional: PageRepoTransactional<C>,
{
}

/// Transactional page repository.
pub trait PageRepoTransactional<C>:
    for<'a> Advance<GetInfoById<'a>, C, Error = RegularError>
    + for<'a> Advance<GetInfoExcluded<'a>, C, Error = RegularError>
    + for<'a> Advance<ListInfosByChapterId<'a>, C, Error = RegularError>
    + for<'a> Advance<ListAllInfosByChapterId<'a>, C, Error = RegularError>
    + for<'a> Advance<CreateBatch<'a>, C, Error = RegularError>
    + for<'a> Advance<ReserveImage<'a>, C, Error = RegularError>
    + for<'a> Advance<MarkImageUploaded<'a>, C, Error = RegularError>
    + for<'a> Advance<SetUnitCounters<'a>, C, Error = RegularError>
    + for<'a> Advance<DeleteByChapterId<'a>, C, Error = RegularError>
{
}
