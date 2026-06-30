//! Value types for member invitation aggregates.

use serde::Deserialize;

/// Include options for member invitation info queries.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum MemberInvitationInclOpt {
    Invitor,
}
