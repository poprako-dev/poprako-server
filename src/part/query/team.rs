use poprako_transactional::advance::Advance;

use crate::part::query::step::team::{
    Create, Delete, GetInfoById, GetInfoExcluded, List, MarkAvatarUploaded, ReserveAvatar,
    UpdateInfo,
};
use crate::part::query::{DeriveTransactional, Execute};
use crate::result::RootError;

pub trait TeamQuery<H>:
    DeriveTransactional
    + for<'a> Execute<Create<'a>, Error = RootError>
    + for<'a> Execute<GetInfoById<'a>, Error = RootError>
    + Execute<List, Error = RootError>
    + for<'a> Execute<UpdateInfo<'a>, Error = RootError>
    + for<'a> Execute<MarkAvatarUploaded<'a>, Error = RootError>
where
    Self::Transactional: TeamQueryTransactional<H>,
{
}

pub trait TeamQueryTransactional<H>:
    for<'a> Advance<ReserveAvatar<'a>, H, Error = RootError>
    + for<'a> Advance<MarkAvatarUploaded<'a>, H, Error = RootError>
    + for<'a> Advance<GetInfoExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<Delete<'a>, H, Error = RootError>
{
}
