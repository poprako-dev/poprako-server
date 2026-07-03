//! Value types for member aggregates.

use serde::Deserialize;

use utoipa::ToSchema;

/// Incl opts for member info queries.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MemberInclOpt {
    User,
    Team,
}
