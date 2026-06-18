use poprako_transactional::advance::Advance;

use crate::part::query::step::team::{
    TeamCreate,
    TeamDelete,
    TeamGetInfoById,
    TeamGetInfoExcluded,
    TeamList,
    TeamMarkAvatarUploaded,
    TeamReserveAvatar,
    TeamUpdateInfo,
};
use crate::part::query::{DeriveTransactional, Execute};
use crate::result::RootError;

pub trait TeamQuery<H>:
    DeriveTransactional
    + for<'a> Execute<TeamCreate<'a>, Error = RootError>
    + for<'a> Execute<TeamGetInfoById<'a>, Error = RootError>
    + Execute<TeamList, Error = RootError>
    + for<'a> Execute<TeamUpdateInfo<'a>, Error = RootError>
    + for<'a> Execute<TeamMarkAvatarUploaded<'a>, Error = RootError>
where
    Self::Transactional: TeamQueryTransactional<H>,
{
}

pub trait TeamQueryTransactional<H>:
    for<'a> Advance<TeamReserveAvatar<'a>, H, Error = RootError>
    + for<'a> Advance<TeamMarkAvatarUploaded<'a>, H, Error = RootError>
    + for<'a> Advance<TeamGetInfoExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<TeamDelete<'a>, H, Error = RootError>
{
}
