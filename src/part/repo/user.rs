use poprako_transactional::advance::Advance;

use crate::part::repo::Execute;
use crate::part::repo::step::user::{
    Create, Delete, GetCredentialByQid, GetInfoById, GetInfoExcluded, MarkAvatarUploaded,
    ReserveAvatar, TouchLastActive, UpdateInfo,
};
use crate::result::RootError;
use crate::util::DeriveTransactional;

pub trait UserRepo<C>:
    DeriveTransactional
    + for<'a> Execute<GetInfoById<'a>, Error = RootError>
    + for<'a> Execute<GetCredentialByQid<'a>, Error = RootError>
where
    Self::Transactional: UserRepoTransactional<C>,
{
}

pub trait UserRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RootError>
    + for<'a> Advance<UpdateInfo<'a>, C, Error = RootError>
    + for<'a> Advance<ReserveAvatar<'a>, C, Error = RootError>
    + for<'a> Advance<MarkAvatarUploaded<'a>, C, Error = RootError>
    + for<'a> Advance<TouchLastActive<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<Delete<'a>, C, Error = RootError>
{
}
