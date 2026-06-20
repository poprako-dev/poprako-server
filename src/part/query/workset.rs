use poprako_transactional::advance::Advance;

use crate::part::query::DeriveTransactional;
use crate::part::query::step::workset::{DeleteCascade, ListByTeamIdExcluded};
use crate::result::RootError;

pub trait WorksetQuery<C>: DeriveTransactional
where
    Self::Transactional: WorksetQueryTransactional<C>,
{
}

pub trait WorksetQueryTransactional<C>:
    for<'a> Advance<ListByTeamIdExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<DeleteCascade<'a>, C, Error = RootError>
{
}
