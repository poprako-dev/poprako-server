use poprako_transactional::advance::Advance;

use crate::part::query::DeriveTransactional;
use crate::part::query::step::member_invitation::{GetInfoByCodeExcluded, MarkPendingAsUsed};
use crate::result::RootError;

pub trait MemberInvitationQuery<H>: DeriveTransactional
where
    Self::Transactional: MemberInvitationQueryTransactional<H>,
{
}

pub trait MemberInvitationQueryTransactional<H>:
    for<'a> Advance<GetInfoByCodeExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<MarkPendingAsUsed<'a>, H, Error = RootError>
{
}
