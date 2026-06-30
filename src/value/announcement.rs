//! Value types for announcement aggregates.

use serde::Deserialize;

/// Include options for announcement info queries.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum AnnouncementInclOpt {
    User,
}
