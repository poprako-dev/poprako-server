//! Value types for member aggregates.

use serde::Deserialize;

/// Include options for member info queries.
#[derive(Deserialize)]
pub enum MemberInclOpt {
    User,
    Team,
}
