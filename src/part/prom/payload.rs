use serde::{Deserialize, Serialize};

use crate::part::prom::payload::chapter::AdvanceRawProvide;
use crate::part::prom::payload::image::Payload as ImagePayload;
use crate::part::prom::payload::invitation::PurgeExpiredInvitation;

/// Deferred chapter payloads.
pub mod chapter;
/// Deferred image payloads.
pub mod image;
/// Deferred invitation payloads.
pub mod invitation;

/// Deferred-action payload grouped by resource domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
// FIXME: bad grouping and naming.
pub enum Payload {
    /// Advance raw provision after every chapter page is uploaded.
    AdvanceRawProvide(AdvanceRawProvide),

    /// Image-domain deferred action.
    Image(ImagePayload),

    /// Purge an invitation when it is still pending at its expiry time.
    PurgeExpiredInvitation(PurgeExpiredInvitation),
}

impl Payload {
    /// Returns the routing topic string (e.g. `"image"`) for this payload.
    pub fn topic(&self) -> &'static str {
        match self {
            //
            Self::AdvanceRawProvide(_) => "advance_raw_provide",

            Self::Image(_) => "image",

            Self::PurgeExpiredInvitation(_) => "purge_expired_invitation",
        }
    }
}
