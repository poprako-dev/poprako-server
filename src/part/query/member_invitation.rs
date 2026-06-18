use poprako_transactional::advance::Advance;

use crate::part::query::step::member_invitation::{
    MemberInvitationGetByCodeExcluded,
    MemberInvitationMarkPendingAsUsed,
};
use crate::part::query::DeriveTransactional;
use crate::result::RootError;

pub trait MemberInvitationQuery<H>: DeriveTransactional
where
    Self::Transactional: MemberInvitationQueryTransactional<H>,
{
}

pub trait MemberInvitationQueryTransactional<H>:
    for<'a> Advance<MemberInvitationGetByCodeExcluded<'a>, H, Error = RootError>
    + for<'a> Advance<MemberInvitationMarkPendingAsUsed<'a>, H, Error = RootError>
{
}
