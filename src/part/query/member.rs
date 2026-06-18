use poprako_transactional::advance::Advance;

use crate::part::query::step::member::{
    MemberCreate,
    MemberDelete,
    MemberListByUserIdExcluded,
    MemberTouchLastActive,
    MemberUpdateUserNickname,
};
use crate::part::query::DeriveTransactional;
use crate::result::RootError;

pub trait MemberQuery<H>: DeriveTransactional
where
    Self::Transactional: MemberQueryTransactional<H>,
{
}

pub trait MemberQueryTransactional<H>:
    for<'a> Advance<MemberCreate<'a>, H, Error = RootError>
    + for<'a> Advance<MemberUpdateUserNickname<'a>, H, Error = RootError>
    + for<'a> Advance<MemberTouchLastActive<'a>, H, Error = RootError>
    + for<'a> Advance<MemberListByUserIdExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<MemberDelete<'a>, H, Error = RootError>
    + Sized
{
}
