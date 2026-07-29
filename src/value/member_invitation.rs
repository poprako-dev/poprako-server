//! Value types for member invitation aggregates.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::incl::InclOpt;

/// Consumption-status filtering mode for listing member invitations.
pub enum MemberInvitationStatus {
    //
    /// Include invitations regardless of consumption status.
    All,

    /// Include only invitations that have not yet been consumed.
    Pending,

    /// Include only invitations that have already been consumed.
    Used,
}

/// Incl opts for member invitation info queries.
///
/// Each opt embeds additional related data into the returned
/// `MemberInvitationInfoView`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MemberInvitationInclOpt {
    /// Embed the user who issued the invitation (`invitor`).
    Invitor,
}

impl InclOpt for MemberInvitationInclOpt {
    // Expand include option to concrete dependency chain.
    fn path(self) -> &'static [Self] {
        match self {
            Self::Invitor => &[Self::Invitor],
        }
    }
}
