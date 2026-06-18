use poprako_transactional::advance::Advance;

use crate::part::query::action::user::{UserGetInfoById, UserUpdInfo};
use crate::part::query::{DeriveTransactional, Execute};
use crate::result::RootError;

pub trait UserQuery<H>:
    DeriveTransactional + for<'a> Execute<UserGetInfoById<'a>, Error = RootError>
where
    Self::Transactional: UserQueryTransactional<H>,
{
}

pub trait UserQueryTransactional<H>:
    for<'a> Advance<UserUpdInfo<'a>, H, Error = RootError>
{
}
