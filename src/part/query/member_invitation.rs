use poprako_transactional::advance::Advance;

use crate::part::query::step::member_invitation::{GetInfoByCodeExcluded, MarkPendingAsUsed};
use crate::result::RootError;
use crate::util::DeriveTransactional;

pub trait MemberInvitationQuery<C>: DeriveTransactional
where
    Self::Transactional: MemberInvitationQueryTransactional<C>,
{
}

pub trait MemberInvitationQueryTransactional<C>:
    for<'a> Advance<GetInfoByCodeExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<MarkPendingAsUsed<'a>, C, Error = RootError>
{
}
