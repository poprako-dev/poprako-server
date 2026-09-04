//! Instr DTOs for the term domain.

//! Request and response DTOs for terminology-entry use cases.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::value::pagination::PubListLimit;

/// Input parameters for creating a terminology entry.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct CreateTermInstr {
    //
    /// Parent terminology base identifier.
    pub termbase_id: String,

    /// Source-language term text.
    pub source: String,
    /// Target-language translations.
    pub targets: Vec<String>,
    /// Optional annotation; absent when no comment is provided.
    pub comment: Option<String>,
}

/// Input parameters for replacing terminology-entry fields.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct UpdateTermInfoInstr {
    //
    /// Identifier of the terminology entry to update.
    pub id: String,

    /// Updated source-language term text.
    pub source: String,
    /// Updated target-language translations.
    pub targets: Vec<String>,
    /// Optional updated annotation; absent when no comment is provided.
    pub comment: Option<String>,
}

/// Input parameters for listing terms inside one terminology base.
#[derive(Debug)]
pub struct ListTermInfosInstr {
    //
    /// Parent terminology base identifier.
    pub termbase_id: String,

    /// Optional fuzzy-search pattern for source-language terms; absent when no term filter is applied.
    pub fuzzy_source: Option<String>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: PubListLimit,
}
