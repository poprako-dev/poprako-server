//! Val DTOs for the member-invitation domain.

//! Data transfer objects for member invitation use cases — input parameters
//! and presentation-ready invitation values.

use serde::Serialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Return value from a successful invitation creation.
///
/// The `code` is a short opaque token the invitee presents during
/// registration to claim the invitation.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateMemberInvitationVal {
    /// Unique identifier of the created invitation.
    pub id: String,
    /// Opaque invitation code presented by the invitee to claim the invitation.
    pub code: String,
}
