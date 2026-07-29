//! Val DTOs for the comic archive domain.

//! Request and response values for immutable comic archive operations.

use std::collections::BTreeMap;

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

/// Value returned after an active comic has been archived atomically.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ArchiveComicVal {
    /// Identifier of the archived comic.
    pub archived_comic_id: String,
}

/// JSON archive payloads grouped by their UTC `YYYY-MM` month slot.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(transparent)]
pub struct ExportComicArchivesVal(pub BTreeMap<String, Vec<String>>);
