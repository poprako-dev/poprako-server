use serde::{Deserialize, Serialize};

/// Deferred cleanup event for an invitation that reached its expiry time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvitationPayload {
    //
    /// Purge a pending assignment invitation.
    Assignment {
        /// ID of the invitation record to purge.
        invitation_id: String,
    },

    /// Purge a pending member invitation.
    Member {
        /// ID of the invitation record to purge.
        invitation_id: String,
    },
}
