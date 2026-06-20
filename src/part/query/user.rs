use poprako_transactional::advance::Advance;

use crate::part::query::step::user::{
    Create, Delete, GetCredentialByQid, GetInfoById, GetInfoExcluded, MarkAvatarUploaded,
    ReserveAvatar, TouchLastActive, UpdateInfo,
};
use crate::part::query::{DeriveTransactional, Execute};
use crate::result::RootError;

pub trait UserQuery<C>:
    DeriveTransactional
    + for<'a> Execute<GetInfoById<'a>, Error = RootError>
    + for<'a> Execute<GetCredentialByQid<'a>, Error = RootError>
where
    Self::Transactional: UserQueryTransactional<C>,
{
}

pub trait UserQueryTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RootError>
    + for<'a> Advance<UpdateInfo<'a>, C, Error = RootError>
    + for<'a> Advance<ReserveAvatar<'a>, C, Error = RootError>
    + for<'a> Advance<MarkAvatarUploaded<'a>, C, Error = RootError>
    + for<'a> Advance<TouchLastActive<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<Delete<'a>, C, Error = RootError>
{
}
