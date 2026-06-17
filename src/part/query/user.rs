use poprako_transactional::advance::Advance;

use crate::part::query::action::user::UpdateInfo;
use crate::result::RootError;

pub trait UserQuery<H> {
    type Transactional: UserQueryTransactional<H>;

    fn transactional(&self) -> Self::Transactional;
}

pub trait UserQueryTransactional<H>:
    for<'a> Advance<UpdateInfo<'a>, H, Error = RootError> + Sized
{
}
