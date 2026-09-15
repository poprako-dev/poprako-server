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
use crate::part::prom::topic::Topic;

/// One deferred task, grouped by its domain.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
pub enum TaskPayload {
    /// Chapter-domain tasks.
    Chapter {
        /// Chapter-domain task payload.
        payload: ChapterPayload,
    },

    /// Invitation-domain tasks.
    Invitation {
        /// Invitation-domain task payload.
        payload: InvitationPayload,
    },
}

impl TaskPayload {
    /// Selects an existing consumption queue for this task.
    /// Tasks in one topic run serially; different topics may run concurrently.
    pub const fn topic(&self) -> Topic {
        //
        match self {
            //
            Self::Chapter { payload } => payload.topic(),

            Self::Invitation { payload } => payload.topic(),
        }
    }
}
