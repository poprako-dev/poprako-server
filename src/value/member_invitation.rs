//! Value types for member invitation aggregates.

use serde::Deserialize;

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

/// Incl opts for member invitation info queries.
///
/// Each opt embeds additional related data into the returned
/// `MemberInvitationInfoVal`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MemberInvitationInclOpt {
    /// Embed the user who issued the invitation (`invitor`).
    Invitor,
}
