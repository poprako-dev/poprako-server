//! Presentation of owned or shared object URL text.

#[cfg(test)]
mod tests;

use std::ops::Deref;
use std::sync::Arc;

use serde::{Serialize, Serializer};

// Single-consumer text stays owned; repeated consumers share the same bytes.
enum ObjUrlText {
    /// Text with one consumer.
    Owned {
        // Buffer moved from the generated URL.
        text: String,
    },

    /// Text shared by repeated consumers.
    Shared {
        // Reference-counted owner of the original buffer.
        text: Arc<String>,
    },
}

/// Object URL text whose allocation can be shared across repeated inclusions.
pub struct ObjUrlView {
    /// Ownership selected by the number of consumers in the presentation batch.
    text: ObjUrlText,
}

impl From<String> for ObjUrlView {
    // Moves a single-consumer buffer.
    fn from(text: String) -> Self {
        //
        Self {
            text: ObjUrlText::Owned { text },
        }
    }
}

impl From<Arc<String>> for ObjUrlView {
    // Moves a shared buffer handle.
    fn from(text: Arc<String>) -> Self {
        //
        Self {
            text: ObjUrlText::Shared { text },
        }
    }
}

impl Deref for ObjUrlView {
    // Exposes the text without revealing its ownership mode.
    type Target = str;

    // Borrows the same text in either ownership mode.
    fn deref(&self) -> &Self::Target {
        //
        match &self.text {
            //
            ObjUrlText::Owned { text } => text.as_str(),

            ObjUrlText::Shared { text } => text.as_str(),
        }
    }
}

impl Serialize for ObjUrlView {
    // Keeps the HTTP representation a string.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self)
    }
}
