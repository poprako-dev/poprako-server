//! Repository traits for the comment domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::comment::{Create, ListInfos};
use crate::part::shared::execute::Execute;
use crate::result::RegularError;
use crate::util::DeriveTransactional;

/// Non-transactional comment repository.
pub trait CommentRepo<C>:
    DeriveTransactional + for<'a> Execute<ListInfos<'a>, Error = RegularError>
where
    Self::Transactional: CommentRepoTransactional<C>,
{
}

/// Transactional comment repository.
pub trait CommentRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RegularError> + Sized
{
}
