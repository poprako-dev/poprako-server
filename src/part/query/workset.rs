use poprako_transactional::advance::Advance;

use crate::part::query::DeriveTransactional;
use crate::part::query::step::workset::{DeleteCascade, ListByTeamIdExcluded};
use crate::result::RootError;

pub trait WorksetQuery<H>: DeriveTransactional
where
    Self::Transactional: WorksetQueryTransactional<H>,
{
}

pub trait WorksetQueryTransactional<H>:
    for<'a> Advance<ListByTeamIdExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<DeleteCascade<'a>, H, Error = RootError>
{
}
