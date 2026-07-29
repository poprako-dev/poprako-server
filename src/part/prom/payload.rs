use serde::{Deserialize, Serialize};

use crate::part::prom::payload::image::Payload as ImagePayload;

/// Deferred image payloads.
pub mod image;

/// Deferred-action payload grouped by resource domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Payload {
    /// Image-domain deferred action.
    Image(ImagePayload),
}

impl Payload {
    /// Returns the routing topic string (e.g. `"image"`) for this payload.
    pub(crate) fn topic(&self) -> &'static str {
        match self {
            Self::Image(_) => "image",
        }
    }
}
