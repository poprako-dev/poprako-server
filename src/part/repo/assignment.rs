//! Repository traits for the assignment domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::assignment::{
    Create, Delete, GetInfoByChapterIdAndUserId, GetInfoById, ListInfos, PutRoles,
};
use crate::part::shared::execute::Execute;
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional assignment repository.
pub trait AssignmentRepo<C>:
    DeriveTransactional
    + for<'a> Execute<GetInfoByChapterIdAndUserId<'a>, Error = RootError>
    + for<'a> Execute<ListInfos<'a>, Error = RootError>
    + for<'a> Execute<GetInfoById<'a>, Error = RootError>
where
    Self::Transactional: AssignmentRepoTransactional<C>,
{
}

/// Transactional assignment repository.
pub trait AssignmentRepoTransactional<C>:
    for<'a> Advance<GetInfoByChapterIdAndUserId<'a>, C, Error = RootError>
    + for<'a> Advance<Create<'a>, C, Error = RootError>
    + for<'a> Advance<PutRoles<'a>, C, Error = RootError>
    + for<'a> Advance<Delete<'a>, C, Error = RootError>
{
}
