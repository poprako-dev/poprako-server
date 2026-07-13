use serde::{Deserialize, Serialize};

use crate::part::prom::payload::image::Payload as ImagePayload;

/// Deferred-action payload grouped by resource domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Payload {
    Image(ImagePayload),
}

impl Payload {
    pub(crate) fn topic(&self) -> &'static str {
        match self {
            Self::Image(_) => "image",
        }
    }
}

/// Deferred image payloads.
pub mod image;
