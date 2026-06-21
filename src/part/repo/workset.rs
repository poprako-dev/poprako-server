use poprako_transactional::advance::Advance;

use crate::part::repo::step::workset::{DeleteCascade, ListByTeamIdExcluded};
use crate::result::RootError;
use crate::util::DeriveTransactional;

pub trait WorksetRepo<C>: DeriveTransactional
where
    Self::Transactional: WorksetRepoTransactional<C>,
{
}

pub trait WorksetRepoTransactional<C>:
    for<'a> Advance<ListByTeamIdExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<DeleteCascade<'a>, C, Error = RootError>
{
}
