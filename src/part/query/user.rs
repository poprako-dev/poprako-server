use poprako_transactional::advance::Advance;

use crate::part::query::step::user::{
    Create, Delete, GetCredentialByQid, GetInfoById, GetInfoExcluded, MarkAvatarUploaded,
    ReserveAvatar, TouchLastActive, UpdateInfo,
};
use crate::part::query::{DeriveTransactional, Execute};
use crate::result::RootError;

pub trait UserQuery<H>:
    DeriveTransactional
    + for<'a> Execute<GetInfoById<'a>, Error = RootError>
    + for<'a> Execute<GetCredentialByQid<'a>, Error = RootError>
where
    Self::Transactional: UserQueryTransactional<H>,
{
}

pub trait UserQueryTransactional<H>:
    for<'a> Advance<Create<'a>, H, Error = RootError>
    + for<'a> Advance<UpdateInfo<'a>, H, Error = RootError>
    + for<'a> Advance<ReserveAvatar<'a>, H, Error = RootError>
    + for<'a> Advance<MarkAvatarUploaded<'a>, H, Error = RootError>
    + for<'a> Advance<TouchLastActive<'a>, H, Error = RootError>
    + for<'a> Advance<GetInfoExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<Delete<'a>, H, Error = RootError>
{
}
