//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

use crate::api::http::result::Wire;
use crate::data::instr::termbase_port::{
    AppendTermbaseInstr, AppendTermbaseTermInstr, ImportTermInstr,
    ImportTermbaseInstr,
};
use crate::data::val::termbase_port::{
    AppendTermbaseVal, ExportTermbaseVal, ImportTermbaseVal,
};
use crate::data::view::termbase_port::TermbaseTermView;

/// One portable terminology entry supplied by an import request.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportTermBody {
    /// Source-language term or phrase.
    pub source: String,
    /// Target-language translations.
    pub targets: Vec<String>,
    /// Optional annotation or usage note.
    pub comment: Option<String>,
}

impl From<ImportTermBody> for ImportTermInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ImportTermBody) -> Self {
        Self {
            source: value.source,
            targets: value.targets,
            comment: value.comment,
        }
    }
}

/// Native terminology-base document supplied to an import endpoint.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportTermbaseBody {
    /// Display name of the terminology base.
    pub name: String,
    /// Optional terminology-base description.
    pub description: Option<String>,
    /// Portable terminology entries to import.
    pub terms: Vec<ImportTermBody>,
}

impl From<ImportTermbaseBody> for ImportTermbaseInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ImportTermbaseBody) -> Self {
        Self {
            name: value.name,
            description: value.description,
            terms: value
                .terms
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

/// One strict portable terminology entry supplied by an append request.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AppendTermbaseTermBody {
    /// Source-language term or phrase.
    pub source: String,
    /// Target-language translations in preferred order.
    pub targets: Vec<String>,
    /// Optional complete annotation block to merge.
    pub comment: Option<String>,
}

impl From<AppendTermbaseTermBody> for AppendTermbaseTermInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AppendTermbaseTermBody) -> Self {
        Self {
            source: value.source,
            targets: value.targets,
            comment: value.comment,
        }
    }
}

/// Strict metadata-free document appended to an existing terminology base.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AppendTermbaseBody {
    /// Portable entries; an empty batch is a successful no-op.
    pub terms: Vec<AppendTermbaseTermBody>,
}

impl From<AppendTermbaseBody> for AppendTermbaseInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AppendTermbaseBody) -> Self {
        Self {
            terms: value
                .terms
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

/// Native portable terminology-base export document.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ExportTermbasePayload {
    /// Display name of the exported terminology base.
    pub name: String,
    /// Optional terminology-base description.
    pub description: Option<String>,
    /// Portable terminology entries ordered deterministically.
    pub terms: Vec<TermbaseTermObj>,
}

impl From<ExportTermbaseVal> for ExportTermbasePayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ExportTermbaseVal) -> Self {
        Self {
            name: value.name,
            description: value.description,
            terms: value
                .terms
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

impl Wire for ExportTermbaseVal {
    // Serializable wire payload produced by this conversion.
    type Target = ExportTermbasePayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Result of importing one native terminology-base document.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportTermbasePayload {
    /// Identifier of the created or merged terminology base.
    pub id: String,
    /// Whether this import created a new terminology base.
    pub created: bool,
    /// Number of new terminology entries created.
    pub created_term_count: usize,
    /// Number of existing terminology entries merged.
    pub merged_term_count: usize,
}

impl From<ImportTermbaseVal> for ImportTermbasePayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ImportTermbaseVal) -> Self {
        Self {
            id: value.id,
            created: value.created,
            created_term_count: value.created_term_count,
            merged_term_count: value.merged_term_count,
        }
    }
}

impl Wire for ImportTermbaseVal {
    // Serializable wire payload produced by this conversion.
    type Target = ImportTermbasePayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Content-change counts for one atomic terminology append.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AppendTermbasePayload {
    /// Number of newly inserted entries.
    pub added: usize,
    /// Number of existing entries whose content changed.
    pub updated: usize,
    /// Number of matched entries whose content was already present.
    pub unchanged: usize,
}

impl From<AppendTermbaseVal> for AppendTermbasePayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AppendTermbaseVal) -> Self {
        Self {
            added: value.added,
            updated: value.updated,
            unchanged: value.unchanged,
        }
    }
}

impl Wire for AppendTermbaseVal {
    // Serializable wire payload produced by this conversion.
    type Target = AppendTermbasePayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Portable terminology-entry content without persistence metadata.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct TermbaseTermObj {
    /// Source-language term or phrase.
    pub source: String,
    /// Target-language translations.
    pub targets: Vec<String>,
    /// Optional annotation or usage note.
    pub comment: Option<String>,
}

impl From<TermbaseTermView> for TermbaseTermObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: TermbaseTermView) -> Self {
        Self {
            source: value.source,
            targets: value.targets,
            comment: value.comment,
        }
    }
}

impl Wire for TermbaseTermView {
    // Serializable wire payload produced by this conversion.
    type Target = TermbaseTermObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Query controls for terminology-base import.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ImportTermbaseQuery {
    /// Merge into a same-name terminology base instead of rejecting it.
    #[serde(default)]
    pub force_merge: bool,
}
