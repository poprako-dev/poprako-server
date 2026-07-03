//! Value types for member invitation aggregates.

use serde::Deserialize;

use utoipa::ToSchema;

/// Incl opts for member invitation info queries.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MemberInvitationInclOpt {
    Invitor,
}
