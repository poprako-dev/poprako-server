use poprako_transactional::advance::Advance;

use crate::part::query::DeriveTransactional;
use crate::part::query::step::member::{
    Create, Delete, ListByUserIdExcluded, TouchLastActive, UpdateUserNickname,
};
use crate::result::RootError;

pub trait MemberQuery<H>: DeriveTransactional
where
    Self::Transactional: MemberQueryTransactional<H>,
{
}

pub trait MemberQueryTransactional<H>:
    for<'a> Advance<Create<'a>, H, Error = RootError>
    + for<'a> Advance<UpdateUserNickname<'a>, H, Error = RootError>
    + for<'a> Advance<TouchLastActive<'a>, H, Error = RootError>
    + for<'a> Advance<ListByUserIdExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<Delete<'a>, H, Error = RootError>
    + Sized
{
}
