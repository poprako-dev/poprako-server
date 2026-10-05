/// Deferred chapter payloads.
pub mod chapter;
/// Deferred invitation payloads.
pub mod invitation;

/// Shared payload tests.
#[cfg(test)]
pub mod tests;

use serde::{Deserialize, Serialize};

use poprako_prom::general::handler::Payload;

use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::invitation::InvitationPayload;
use crate::part::prom::topic::Topic;

/// One deferred task, grouped by its domain.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(Debug))]
pub enum PromPayload {
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

impl PromPayload {
    /// Selects an existing consumption queue for this task.
    /// Tasks in one topic run serially; different topics may run concurrently.
    #[must_use]
    pub const fn topic(&self) -> Topic {
        //
        match self {
            //
            Self::Chapter { payload } => payload.topic(),

            Self::Invitation { payload } => payload.topic(),
        }
    }
}

impl Payload for PromPayload {
    // Registered topics for the application payload.
    const TOPICS: &'static [&'static str] =
        &[Topic::Chapter.as_str(), Topic::Invitation.as_str()];

    // Returns the routing topic for this payload.
    fn topic(&self) -> &'static str {
        self.topic().as_str()
    }
}
