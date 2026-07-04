//! Value types for announcement aggregates.

use serde::Deserialize;

use utoipa::ToSchema;

/// Incl opts for announcement info queries.
///
/// Each opt embeds additional related data into the returned
/// `AnnouncementInfoVal`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementInclOpt {
    /// Embed the announcement's author (`user`).
    User,
}
