//! Process-local coordination of immutable annotation save batches.

use uuid::Uuid;

use crate::value::issue_save::IssueSaveReceipt;

/// Exclusion spanning receipt lookup, transaction commit, and publication.
pub trait IssueSaveGuard {
    /// Completed request fingerprint and assigned identities, when retained.
    fn receipt(&self) -> Option<IssueSaveReceipt>;
    /// Records successful completion after the database commit.
    fn record(&self, receipt: IssueSaveReceipt);
}

/// Shared bounded receipt cache.
pub trait IssueSaveCache {
    /// Adapter-owned guard.
    type Guard<'a>: IssueSaveGuard
    where
        Self: 'a;
    /// Acquires exclusion for this batch UUID.
    fn lock_issue_save(
        &self,
        save_id: Uuid,
    ) -> impl Future<Output = Self::Guard<'_>> + Send;
}
