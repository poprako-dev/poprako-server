//! Best-effort event queue configuration.

use std::num::NonZeroUsize;

use anyhow::ensure;
use serde::Deserialize;
use tokio::sync::Semaphore;

/// Capacity of the shared best-effort event queue.
#[derive(Clone, Copy, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EffectConfig {
    /// Maximum number of events waiting for the consumer.
    #[serde(rename = "queue_capacity")]
    pub capacity: NonZeroUsize,
}

impl EffectConfig {
    /// Rejects capacities unsupported by Tokio's bounded channel.
    ///
    /// # Errors
    /// Returns an error when the capacity exceeds Tokio's semaphore limit.
    pub fn validate(self) -> anyhow::Result<()> {
        //
        ensure!(
            self.capacity.get() <= Semaphore::MAX_PERMITS,
            "effect.queue_capacity exceeds the channel capacity range"
        );

        Ok(())
    }
}

impl Default for EffectConfig {
    // Supplies the default runtime policy.
    fn default() -> Self {
        Self {
            capacity: NonZeroUsize::MIN.saturating_add(1023),
        }
    }
}
