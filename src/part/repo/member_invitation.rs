//! Repository traits for the member-invitation domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::member_invitation::{
    CreateMemberInvitation, DeleteMemberInvitation, GetMemberInvitationInfo,
    GetMemberInvitationInfoExcluded, ListMemberInvitationInfos,
    PurgeExpiredMemberInvitation, UpdateMemberInvitation,
};
use crate::result::RegularError;

/// Member-invitation repository operations.
///
/// Standalone reads use [`Run`]. Transactional reads, mutations, and locks
/// use [`Step`] with the context coordinated by the caller.
pub trait MemberInvitationRepo<C>:
    for<'a> Run<ListMemberInvitationInfos<'a>, Error = RegularError>
    + for<'a, 'b> Run<GetMemberInvitationInfo<'a, 'b>, Error = RegularError>
    + for<'a> Step<CreateMemberInvitation<'a>, C, Error = RegularError>
    + for<'a, 'b> Step<GetMemberInvitationInfo<'a, 'b>, C, Error = RegularError>
    + for<'a> Step<UpdateMemberInvitation<'a>, C, Error = RegularError>
    + for<'a> Step<GetMemberInvitationInfoExcluded<'a>, C, Error = RegularError>
    + for<'a> Step<PurgeExpiredMemberInvitation<'a>, C, Error = RegularError>
    + for<'a> Step<DeleteMemberInvitation<'a>, C, Error = RegularError>
{
}
