//! Val DTOs for the member-invitation domain.
//! Data transfer objects for member invitation use cases — input parameters
//! and presentation-ready invitation values.
/// Return value from a successful invitation creation.
///
/// The `code` is a short opaque token the invitee presents during
/// registration to claim the invitation.
pub struct CreateMemberInvitationVal {
    /// Unique identifier of the created invitation.
    pub id: String,
    /// Opaque invitation code presented by the invitee to claim the invitation.
    pub code: String,
}
