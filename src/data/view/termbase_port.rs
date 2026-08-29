//! Response-neutral fragments for native terminology-base export.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::model::read::proj::term::TermInfo;

/// Portable terminology-entry content without persistence metadata.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct TermbaseTermView {
    /// Source-language term or phrase.
    pub source: String,
    /// Target-language translations.
    pub targets: Vec<String>,
    /// Optional annotation or usage note.
    pub comment: Option<String>,
}

impl From<TermInfo> for TermbaseTermView {
    // Convert persisted term information into portable response content.
    fn from(term_info: TermInfo) -> Self {
        //
        Self {
            source: term_info.source,
            targets: term_info.targets,
            comment: term_info.comment,
        }
    }
}
