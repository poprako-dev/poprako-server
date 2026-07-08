//! Repository traits for the assignment domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::assignment::{
    Create, Delete, DeleteByChapterId, GetInfoByChapterIdAndUserId, GetInfoById,
    ListAllInfosByChapter, ListInfos, ListInfosByChapterIdExcluded, PutRoles,
};
use crate::part::shared::execute::Execute;
use crate::result::RegularError;
use crate::util::DeriveTransactional;

/// Non-transactional assignment repository.
pub trait AssignmentRepo<C>:
    DeriveTransactional
    + for<'a> Execute<GetInfoByChapterIdAndUserId<'a>, Error = RegularError>
    + for<'a> Execute<ListInfos<'a>, Error = RegularError>
    + for<'a> Execute<GetInfoById<'a>, Error = RegularError>
    + for<'a> Execute<ListAllInfosByChapter<'a>, Error = RegularError>
where
    Self::Transactional: AssignmentRepoTransactional<C>,
{
}

/// Transactional assignment repository.
pub trait AssignmentRepoTransactional<C>:
    for<'a> Advance<GetInfoByChapterIdAndUserId<'a>, C, Error = RegularError>
    + for<'a> Advance<ListInfosByChapterIdExcluded<'a>, C, Error = RegularError>
    + for<'a> Advance<ListAllInfosByChapter<'a>, C, Error = RegularError>
    + for<'a> Advance<Create<'a>, C, Error = RegularError>
    + for<'a> Advance<PutRoles<'a>, C, Error = RegularError>
    + for<'a> Advance<Delete<'a>, C, Error = RegularError>
    + for<'a> Advance<DeleteByChapterId<'a>, C, Error = RegularError>
{
}
