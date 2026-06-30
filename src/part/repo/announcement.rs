//! Repository traits for the announcement domain.

use poprako_transactional::advance::Advance;

use crate::part::repo::step::announcement::{Create, ListInfos};
use crate::part::shared::execute::Execute;
use crate::result::RootError;
use crate::util::DeriveTransactional;

/// Non-transactional announcement repository.
pub trait AnnouncementRepo<C>:
    DeriveTransactional + for<'a> Execute<ListInfos<'a>, Error = RootError>
where
    Self::Transactional: AnnouncementRepoTransactional<C>,
{
}

/// Transactional announcement repository.
pub trait AnnouncementRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RootError> + Sized
{
}
