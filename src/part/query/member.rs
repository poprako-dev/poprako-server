use poprako_transactional::advance::Advance;

use crate::part::query::DeriveTransactional;
use crate::part::query::step::member::{
    Create, Delete, ListByUserIdExcluded, TouchLastActive, UpdateUserNickname,
};
use crate::result::RootError;

pub trait MemberQuery<C>: DeriveTransactional
where
    Self::Transactional: MemberQueryTransactional<C>,
{
}

pub trait MemberQueryTransactional<C>:
    for<'a> Advance<Create<'a>, C, Error = RootError>
    + for<'a> Advance<UpdateUserNickname<'a>, C, Error = RootError>
    + for<'a> Advance<TouchLastActive<'a>, C, Error = RootError>
    + for<'a> Advance<ListByUserIdExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<Delete<'a>, C, Error = RootError>
    + Sized
{
}
