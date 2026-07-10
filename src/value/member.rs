//! Value types for member aggregates.

use serde::Deserialize;

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

/// Incl opts for member info queries.
///
/// Each opt embeds additional related data into the returned
/// `MemberInfoVal`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum MemberInclOpt {
    /// Embed the member's user profile (`user`).
    User,
    /// Embed the member's team (`team`).
    Team,
}
