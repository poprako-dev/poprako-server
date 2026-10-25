//! Stable identities assigned by an annotation save.

use std::sync::Arc;

/// One temporary annotation identity and its committed business identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueSaveCreatedIssueId {
    /// Client identity scoped to this immutable save batch.
    pub local_id: String,
    /// Permanent server-generated annotation identity.
    pub issue_id: String,
}

/// Successful bounded save receipt published after transaction commit.
#[derive(Clone)]
pub struct IssueSaveReceipt {
    /// Fingerprint of the actor, target page and submitted document.
    pub fingerprint: [u8; 32],
    /// Identities created by this exact batch.
    pub created_issue_ids: Arc<[IssueSaveCreatedIssueId]>,
}
