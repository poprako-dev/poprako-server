//! Value types for announcement aggregates.

use serde::Deserialize;

use utoipa::ToSchema;

/// Incl opts for announcement info queries.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementInclOpt {
    User,
}
