//! Repository traits for the assignment domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::Execute;
use crate::part::repo::step::assignment::{Create, DeleteByChapter, GetByChapterUserId, PutRoles};
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional assignment repository.
pub trait AssignmentRepo<C>:
    DeriveTransactional + for<'a> Execute<GetByChapterUserId<'a>, Error = RootError>
where
    Self::Transactional: AssignmentRepoTransactional<C>,
{
}

/// Transactional assignment repository.
pub trait AssignmentRepoTransactional<C>:
    for<'a> Advance<GetByChapterUserId<'a>, C, Error = RootError>
    + for<'a> Advance<Create<'a>, C, Error = RootError>
    + for<'a> Advance<PutRoles<'a>, C, Error = RootError>
    + for<'a> Advance<DeleteByChapter<'a>, C, Error = RootError>
{
}
