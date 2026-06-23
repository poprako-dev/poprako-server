use poprako_transactional::advance::Advance;

use crate::part::repo::step::member_invitation::{GetInfoByCodeExcluded, MarkPendingAsUsed};
use crate::result::RootError;
use crate::util::DeriveTransactional;

pub trait MemberInvitationRepo<C>: DeriveTransactional
where
    Self::Transactional: MemberInvitationRepoTransactional<C>,
{
}

pub trait MemberInvitationRepoTransactional<C>:
    for<'a> Advance<GetInfoByCodeExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<MarkPendingAsUsed<'a>, C, Error = RootError>
{
}
