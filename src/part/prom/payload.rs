use serde::{Deserialize, Serialize};

use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::payload::image::ImagePayload;
use crate::part::prom::payload::invitation::InvitationPayload;

/// Deferred chapter payloads.
pub mod chapter;
/// Deferred image payloads.
pub mod image;
/// Deferred invitation payloads.
pub mod invitation;
/// Shared payload tests.
#[cfg(test)]
pub mod tests;

/// One deferred task, grouped by its domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPayload {
    //
    /// Advance raw provision after every chapter page is uploaded.
    #[serde(rename = "AdvanceRawProvide")]
    Chapter(ChapterPayload),

    /// Image-domain deferred action.
    Image(ImagePayload),

    /// Purge an invitation when it is still pending at its expiry time.
    #[serde(rename = "PurgeExpiredInvitation")]
    Invitation(InvitationPayload),
}

impl TaskPayload {
    /// Returns the routing topic string (e.g. `"image"`) for this payload.
    pub fn topic(&self) -> &'static str {
        //
        match self {
            //
            Self::Chapter(_) => "advance_raw_provide",

            Self::Image(_) => "image",

            Self::Invitation(_) => "purge_expired_invitation",
        }
    }
}
