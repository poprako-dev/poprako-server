use poprako_transactional::advance::Advance;

use crate::part::repo::step::member::{
    Create, Delete, ListByUserIdExcluded, TouchLastActive, UpdateUserNickname,
};
use crate::result::RootError;
use crate::util::DeriveTransactional;

pub trait MemberRepo<C>: DeriveTransactional
where
    Self::Transactional: MemberRepoTransactional<C>,
{
}

pub trait MemberRepoTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RootError>
    + for<'a> Advance<UpdateUserNickname<'a>, C, Error = RootError>
    + for<'a> Advance<TouchLastActive<'a>, C, Error = RootError>
    + for<'a> Advance<ListByUserIdExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<Delete<'a>, C, Error = RootError>
    + Sized
{
}
