//! Value types for member invitation aggregates.

use serde::Deserialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::incl::InclOpt;

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
        //
        match self {
            Self::Invitor => &[Self::Invitor],
        }
    }
}
