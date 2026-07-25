//! Repository traits for the member-invitation domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::member_invitation::{CreateMemberInvitation, DeleteMemberInvitation, GetMemberInvitationInfo, GetMemberInvitationInfoExcluded, ListMemberInvitationInfos, PurgeExpiredMemberInvitation, UpdateMemberInvitation};
use crate::result::BaseError;

/// Member-invitation repository operations.
///
/// Standalone reads use [`Run`]. Transactional reads, mutations, and locks
/// use [`Step`] with the context coordinated by the caller.
pub trait MemberInvitationRepo<C>:
    for<'a> Run<ListMemberInvitationInfos<'a>, Error = BaseError>
    + for<'a, 'b> Run<GetMemberInvitationInfo<'a, 'b>, Error = BaseError>
    + for<'a> Run<PurgeExpiredMemberInvitation<'a>, Error = BaseError>
    + for<'a> Step<CreateMemberInvitation<'a>, C, Error = BaseError>
    + for<'a, 'b> Step<GetMemberInvitationInfo<'a, 'b>, C, Error = BaseError>
    + for<'a> Step<UpdateMemberInvitation<'a>, C, Error = BaseError>
    + for<'a> Step<GetMemberInvitationInfoExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<PurgeExpiredMemberInvitation<'a>, C, Error = BaseError>
    + for<'a> Step<DeleteMemberInvitation<'a>, C, Error = BaseError>
{
}

impl<T, C> MemberInvitationRepo<C> for T where
    T: for<'a> Run<ListMemberInvitationInfos<'a>, Error = BaseError>
        + for<'a, 'b> Run<GetMemberInvitationInfo<'a, 'b>, Error = BaseError>
        + for<'a> Run<PurgeExpiredMemberInvitation<'a>, Error = BaseError>
        + for<'a> Step<CreateMemberInvitation<'a>, C, Error = BaseError>
        + for<'a, 'b> Step<GetMemberInvitationInfo<'a, 'b>, C, Error = BaseError>
        + for<'a> Step<UpdateMemberInvitation<'a>, C, Error = BaseError>
        + for<'a> Step<GetMemberInvitationInfoExcluded<'a>, C, Error = BaseError>
        + for<'a> Step<PurgeExpiredMemberInvitation<'a>, C, Error = BaseError>
        + for<'a> Step<DeleteMemberInvitation<'a>, C, Error = BaseError>
{
}
