//! Value types for member aggregates.

use serde::Deserialize;

/// Include options for member info queries.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum MemberInclOpt {
    User,
    Team,
}
