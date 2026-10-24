//! Process-local coordination of immutable annotation save batches.

use uuid::Uuid;

/// Exclusion spanning receipt lookup, transaction commit, and publication.
pub trait IssueSaveGuard {
    /// Fingerprint of the completed request, when still retained.
    fn fingerprint(&self) -> Option<[u8; 32]>;
    /// Records successful completion after the database commit.
    fn record(&self, fingerprint: [u8; 32]);
}

/// Shared bounded receipt cache.
pub trait IssueSaveRepo {
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
