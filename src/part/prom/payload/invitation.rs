use serde::{Deserialize, Serialize};

/// Deferred cleanup event for an invitation that reached its expiry time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PurgeExpiredInvitation {
    /// Purge a pending assignment invitation.
    Assignment { invitation_id: String },

    /// Purge a pending member invitation.
    Member { invitation_id: String },
}
