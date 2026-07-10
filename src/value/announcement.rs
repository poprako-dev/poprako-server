//! Value types for announcement aggregates.

use serde::Deserialize;

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

/// Incl opts for announcement info queries.
///
/// Each opt embeds additional related data into the returned
/// `AnnouncementInfoVal`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementInclOpt {
    /// Embed the announcement's author (`user`).
    User,
}
