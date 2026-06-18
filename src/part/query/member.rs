use poprako_transactional::advance::Advance;

use crate::part::query::action::member::MemberUpdUserNickname;
use crate::result::RootError;

pub trait MemberQuery<H> {
    type Transactional: MemberQueryTransactional<H>;

    fn transactional(&self) -> Self::Transactional;
}

pub trait MemberQueryTransactional<H>:
    for<'a> Advance<MemberUpdUserNickname<'a>, H, Error = RootError> + Sized
{
}
