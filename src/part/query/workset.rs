use poprako_transactional::advance::Advance;

use crate::part::query::step::workset::{
    WorksetDeleteCascade,
    WorksetListByTeamIdExcluded,
};
use crate::part::query::DeriveTransactional;
use crate::result::RootError;

pub trait WorksetQuery<H>: DeriveTransactional
where
    Self::Transactional: WorksetQueryTransactional<H>,
{
}

pub trait WorksetQueryTransactional<H>:
    for<'a> Advance<WorksetListByTeamIdExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<WorksetDeleteCascade<'a>, H, Error = RootError>
{
}
