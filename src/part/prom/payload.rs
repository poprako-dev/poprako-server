/// Deferred chapter payloads.
pub mod chapter;
/// Deferred image payloads.
pub mod image;
/// Deferred invitation payloads.
pub mod invitation;

/// Shared payload tests.
#[cfg(test)]
pub mod tests;

use serde::{Deserialize, Serialize};

use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::image::ImagePayload;
use crate::part::prom::payload::invitation::InvitationPayload;

/// One deferred task, grouped by its domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPayload {
    //
    /// Advance raw provision after every chapter page is uploaded.
    #[serde(rename = "AdvanceRawProvide")]
    Chapter {
        /// Chapter-domain task payload.
        #[serde(flatten)]
        payload: ChapterPayload,
    },

    /// Image-domain deferred action.
    Image {
        /// Image-domain task payload.
        #[serde(flatten)]
        payload: ImagePayload,
    },

    /// Purge an invitation when it is still pending at its expiry time.
    #[serde(rename = "PurgeExpiredInvitation")]
    Invitation {
        /// Invitation-domain task payload.
        #[serde(flatten)]
        payload: InvitationPayload,
    },
}

impl TaskPayload {
    /// Returns the routing topic string (e.g. `"image"`) for this payload.
    pub const fn topic(&self) -> &'static str {
        //
        match self {
            //
            Self::Chapter { .. } => "advance_raw_provide",

            Self::Image { .. } => "image",

            Self::Invitation { .. } => "purge_expired_invitation",
        }
    }
}
