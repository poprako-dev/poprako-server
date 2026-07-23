//! Request and response values for immutable comic archive operations.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

/// Value returned after an active comic has been archived atomically.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ArchiveComicPayload {
    pub archived_comic_id: String,
}

/// Query parameters for exporting selected retained month slots.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ExportComicArchivesParams {
    #[serde(
        default,
        rename = "month",
        deserialize_with = "crate::value::query::deserialize_vec"
    )]
    pub months: Vec<String>,
}

/// JSON archive payloads grouped by their UTC `YYYY-MM` month slot.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
#[serde(transparent)]
pub struct ExportComicArchivesPayload(pub BTreeMap<String, Vec<String>>);
