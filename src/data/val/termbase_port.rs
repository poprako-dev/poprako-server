//! Response DTOs for native terminology-base import and export.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::data::view::termbase_port::TermbaseTermView;
use crate::model::read::proj::term::TermInfo;
use crate::model::read::proj::termbase::TermbaseInfo;

/// Native portable terminology-base export document.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ExportTermbaseVal {
    //
    /// Display name of the exported terminology base.
    pub name: String,
    /// Optional terminology-base description.
    pub description: Option<String>,
    /// Portable terminology entries ordered deterministically.
    pub terms: Vec<TermbaseTermView>,
}

impl ExportTermbaseVal {
    /// Build an export response from persisted terminology-base models.
    pub fn from_models(
        termbase_info: TermbaseInfo,
        term_infos: Vec<TermInfo>,
    ) -> Self {
        //
        Self {
            name: termbase_info.name,
            description: termbase_info.description,
            terms: term_infos.into_iter().map(Into::into).collect(),
        }
    }
}

/// Result of importing one native terminology-base document.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportTermbaseVal {
    //
    /// Identifier of the created or merged terminology base.
    pub id: String,
    /// Whether this import created a new terminology base.
    pub created: bool,
    /// Number of new terminology entries created.
    pub created_term_count: i32,
    /// Number of existing terminology entries merged.
    pub merged_term_count: i32,
}
