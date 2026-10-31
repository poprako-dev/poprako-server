//! Resource budgets for complete comic archive exports.

use std::num::NonZeroU32;

use anyhow::ensure;
use serde::Deserialize;
use tokio::sync::Semaphore;

/// Per-export memory and disk budgets and process-wide admission policy.
#[derive(Clone, Copy, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ComicArchiveExportConfig {
    /// Maximum combined stored payload bytes materialized per batch, in MiB.
    pub payload_batch_mib: NonZeroU32,
    /// Maximum serialized month-map artifact size, in MiB.
    /// Excludes the fixed HTTP envelope.
    pub artifact_limit_mib: NonZeroU32,

    /// Maximum artifacts being generated or transferred simultaneously.
    pub concurrency: NonZeroU32,
}

impl ComicArchiveExportConfig {
    /// Returns the byte budget for one materialized database batch.
    #[must_use]
    pub fn payload_batch_bytes(self) -> u64 {
        u64::from(self.payload_batch_mib.get()) * 1024 * 1024
    }

    /// Returns the byte budget for the complete serialized artifact.
    #[must_use]
    pub fn artifact_limit_bytes(self) -> u64 {
        u64::from(self.artifact_limit_mib.get()) * 1024 * 1024
    }

    /// Rejects admission capacities unsupported by Tokio.
    ///
    /// # Errors
    /// Returns an error when concurrency exceeds the platform semaphore capacity.
    pub fn validate(self) -> anyhow::Result<()> {
        //
        ensure!(
            usize::try_from(self.concurrency.get())
                .is_ok_and(|capacity| capacity <= Semaphore::MAX_PERMITS),
            "comic_archive_export.concurrency exceeds semaphore capacity"
        );

        Ok(())
    }
}

impl Default for ComicArchiveExportConfig {
    // Supplies conservative budgets for a small production host.
    fn default() -> Self {
        Self {
            payload_batch_mib: NonZeroU32::MIN.saturating_add(7),
            artifact_limit_mib: NonZeroU32::MIN.saturating_add(255),
            concurrency: NonZeroU32::MIN,
        }
    }
}
