use serde::{Deserialize, Serialize};

use crate::part::prom::payload::image::Payload as ImagePayload;
use crate::part::prom::payload::invitation::PurgeExpiredInvitation;

/// Deferred image payloads.
pub mod image;
/// Deferred invitation payloads.
pub mod invitation;

/// Deferred-action payload grouped by resource domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Payload {
    /// Image-domain deferred action.
    Image(ImagePayload),

    /// Purge an invitation when it is still pending at its expiry time.
    PurgeExpiredInvitation(PurgeExpiredInvitation),
}

impl Payload {
    /// Returns the routing topic string (e.g. `"image"`) for this payload.
    pub(crate) fn topic(&self) -> &'static str {
        match self {
            Self::Image(_) => "image",

            Self::PurgeExpiredInvitation(_) => "purge_expired_invitation",
        }
    }
}
