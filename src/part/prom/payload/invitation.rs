use serde::{Deserialize, Serialize};

use crate::part::prom::topic::Topic;

/// Deferred invitation task payload.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
pub enum InvitationPayload {
    /// Purge a pending assignment invitation.
    PurgeExpiredAssignmentInvitation {
        /// ID of the invitation record to purge.
        invitation_id: String,
    },

    /// Purge a pending member invitation.
    PurgeExpiredMemberInvitation {
        /// ID of the invitation record to purge.
        invitation_id: String,
    },
}

impl InvitationPayload {
    /// Selects a shared queue for these invitation operations.
    pub const fn topic(&self) -> Topic {
        //
        match self {
            //
            Self::PurgeExpiredAssignmentInvitation { .. }
            | Self::PurgeExpiredMemberInvitation { .. } => Topic::Invitation,
        }
    }
}
