use poprako_transactional::advance::Advance;

use crate::part::query::step::user::{
    UserCreate,
    UserDelete,
    UserGetCredentialByQid,
    UserGetInfoById,
    UserGetInfoExcluded,
    UserMarkAvatarUploaded,
    UserReserveAvatar,
    UserTouchLastActive,
    UserUpdateInfo,
};
use crate::part::query::{DeriveTransactional, Execute};
use crate::result::RootError;

pub trait UserQuery<H>:
    DeriveTransactional
    + for<'a> Execute<UserGetInfoById<'a>, Error = RootError>
    + for<'a> Execute<UserGetCredentialByQid<'a>, Error = RootError>
where
    Self::Transactional: UserQueryTransactional<H>,
{
}

pub trait UserQueryTransactional<H>:
    for<'a> Advance<UserCreate<'a>, H, Error = RootError>
    + for<'a> Advance<UserUpdateInfo<'a>, H, Error = RootError>
    + for<'a> Advance<UserReserveAvatar<'a>, H, Error = RootError>
    + for<'a> Advance<UserMarkAvatarUploaded<'a>, H, Error = RootError>
    + for<'a> Advance<UserTouchLastActive<'a>, H, Error = RootError>
    + for<'a> Advance<UserGetInfoExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<UserDelete<'a>, H, Error = RootError>
{
}
