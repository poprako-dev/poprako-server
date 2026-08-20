//! Instr DTOs for the comic archive domain.

//! Request and response values for immutable comic archive operations.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::IntoParams;

/// Query parameters for exporting selected archive month slots.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ExportComicArchivesInstr {
    #[serde(default, rename = "month")]
    /// UTC month slots in `YYYY-MM` format to filter archives by.
    pub months: Vec<String>,
}
