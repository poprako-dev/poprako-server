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
    #[serde(deserialize_with = "duration::deserialize")]
    pub member_ttl_secs: Duration,
    /// Delay before cleaning up an unconsumed assignment invitation.
    #[serde(deserialize_with = "duration::deserialize")]
    pub assignment_ttl_secs: Duration,
}

impl InvitationConfig {
    /// Validates delays against the deferred-task timestamp range.
    ///
    /// # Errors
    /// Returns an error for zero delays or unrepresentable visibility times.
    pub fn validate(self) -> anyhow::Result<()> {
        //
        for (field, duration) in [
            ("member_ttl_secs", self.member_ttl_secs),
            ("assignment_ttl_secs", self.assignment_ttl_secs),
        ] {
            //
            ensure!(
                !duration.is_zero(),
                "invitation.{} must be positive",
                field
            );

            let delay = time::Duration::try_from(duration)?;

            ensure!(
                OffsetDateTime::now_utc().checked_add(delay).is_some(),
                "invitation.{} exceeds the deferred-task timestamp range",
                field
            );
        }

        Ok(())
    }
}

impl Default for InvitationConfig {
    // Supplies the default runtime policy.
    fn default() -> Self {
        Self {
            member_ttl_secs: Duration::from_hours(120),
            assignment_ttl_secs: Duration::from_hours(72),
        }
    }
}
