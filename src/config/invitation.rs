//! Expiration delays for newly created invitations.

use std::time::Duration;

use anyhow::ensure;
use serde::Deserialize;
use time::OffsetDateTime;

use crate::config::duration;

/// Delays before pending invitations become eligible for cleanup.
#[derive(Clone, Copy, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InvitationConfig {
    /// Delay before cleaning up an unconsumed member invitation.
    #[serde(
        rename = "member_ttl_secs",
        deserialize_with = "duration::deserialize"
    )]
    pub member_ttl: Duration,
    /// Delay before cleaning up an unconsumed assignment invitation.
    #[serde(
        rename = "assignment_ttl_secs",
        deserialize_with = "duration::deserialize"
    )]
    pub assignment_ttl: Duration,
}

impl InvitationConfig {
    /// Validates delays against the deferred-task timestamp range.
    ///
    /// # Errors
    /// Returns an error for zero delays or unrepresentable visibility times.
    pub fn validate(self) -> anyhow::Result<()> {
        //
        for (field, duration) in [
            ("member_ttl_secs", self.member_ttl),
            ("assignment_ttl_secs", self.assignment_ttl),
        ] {
            //
            ensure!(!duration.is_zero(), "invitation.{field} must be positive");

            let delay = time::Duration::try_from(duration)?;

            ensure!(
                OffsetDateTime::now_utc().checked_add(delay).is_some(),
                "invitation.{field} exceeds the deferred-task timestamp range"
            );
        }

        Ok(())
    }
}

impl Default for InvitationConfig {
    // Supplies the default runtime policy.
    fn default() -> Self {
        Self {
            member_ttl: Duration::from_hours(120),
            assignment_ttl: Duration::from_hours(72),
        }
    }
}
