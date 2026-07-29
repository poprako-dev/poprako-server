//! Repository traits for the member-invitation domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::member_invitation::{
    CreateMemberInvitation, DeleteMemberInvitation, GetMemberInvitationInfo,
    GetMemberInvitationInfoExcluded, ListMemberInvitationInfos,
    PurgeExpiredMemberInvitation, UpdateMemberInvitation,
};
use crate::result::BaseError;

/// Member-invitation repository operations.
///
/// Standalone reads use [`poprako_orchestra::Run`]. Transactional reads, mutations, and locks
/// use [`poprako_orchestra::Step`] with the context coordinated by the caller.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> ListMemberInvitationInfos<'a>,
        for<'a, 'b> GetMemberInvitationInfo<'a, 'b>,
        for<'a> PurgeExpiredMemberInvitation<'a>,
    ),
    step(
        for<'a> CreateMemberInvitation<'a>,
        for<'a, 'b> GetMemberInvitationInfo<'a, 'b>,
        for<'a> UpdateMemberInvitation<'a>,
        for<'a> GetMemberInvitationInfoExcluded<'a>,
        for<'a> PurgeExpiredMemberInvitation<'a>,
        for<'a> DeleteMemberInvitation<'a>,
    ),
)]
pub trait MemberInvitationRepo<C> {}
