//! Value types for announcement aggregates.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::incl::InclOpt;

/// Incl opts for announcement info queries.
///
/// Each opt embeds additional related data into the returned
/// `AnnouncementInfoVal`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum AnnouncementInclOpt {
    /// Embed the announcement's author (`user`).
    User,
}

impl InclOpt for AnnouncementInclOpt {
    fn path(self) -> &'static [Self] {
        match self {
            Self::User => &[Self::User],
        }
    }
}
