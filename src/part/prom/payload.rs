/// Deferred chapter payloads.
pub mod chapter;
/// Deferred invitation payloads.
pub mod invitation;

/// Shared payload tests.
#[cfg(test)]
pub mod tests;

use serde::{Deserialize, Serialize};

use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::invitation::InvitationPayload;

/// One deferred task, grouped by its domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPayload {
    //
    /// Advance raw provision after every chapter page is uploaded.
    Chapter {
        /// Chapter-domain task payload.
        payload: ChapterPayload,
    },

    /// Purge an invitation when it is still pending at its expiry time.
    Invitation {
        /// Invitation-domain task payload.
        payload: InvitationPayload,
    },
}

impl TaskPayload {
    /// Returns the task category used as its consumption topic.
    /// Tasks in one topic run serially; different topics may run concurrently.
    pub const fn topic(&self) -> &'static str {
        //
        match self {
            //
            Self::Chapter { .. } => "advance_raw_provide",

            Self::Invitation { .. } => "purge_expired_invitation",
        }
    }
}
