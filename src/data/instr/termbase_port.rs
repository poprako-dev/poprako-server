//! Request DTOs for native terminology-base import.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::model::write::term::TermImport;
use crate::model::write::termbase::TermbaseImport;

/// One portable terminology entry supplied by an import request.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportTermInstr {
    /// Source-language term or phrase.
    pub source: String,
    /// Target-language translations.
    pub targets: Vec<String>,
    /// Optional annotation or usage note.
    pub comment: Option<String>,
}

impl<'a> From<&'a ImportTermInstr> for TermImport<'a> {
    // Convert one import entry into the domain write model.
    fn from(instr: &'a ImportTermInstr) -> Self {
        Self {
            source: &instr.source,
            targets: instr.targets.iter().map(String::as_str).collect(),
            comment: instr.comment.as_deref(),
        }
    }
}

/// Native terminology-base document supplied to an import endpoint.
#[derive(Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportTermbaseInstr {
    /// Display name of the terminology base.
    pub name: String,
    /// Optional terminology-base description.
    pub description: Option<String>,
    /// Portable terminology entries to import.
    pub terms: Vec<ImportTermInstr>,
}

impl<'a> From<&'a ImportTermbaseInstr> for TermbaseImport<'a> {
    // Convert the complete import document into the domain write model.
    fn from(instr: &'a ImportTermbaseInstr) -> Self {
        Self {
            name: &instr.name,
            description: instr.description.as_deref(),
            terms: instr.terms.iter().map(Into::into).collect(),
        }
    }
}

/// One strict portable terminology entry supplied by an append request.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AppendTermbaseTermInstr {
    /// Source-language term or phrase.
    pub source: String,
    /// Target-language translations in preferred order.
    pub targets: Vec<String>,
    /// Optional complete annotation block to merge.
    pub comment: Option<String>,
}

impl<'a> From<&'a AppendTermbaseTermInstr> for TermImport<'a> {
    // Convert append content without introducing terminology-base metadata.
    fn from(instr: &'a AppendTermbaseTermInstr) -> Self {
        Self {
            source: &instr.source,
            targets: instr.targets.iter().map(String::as_str).collect(),
            comment: instr.comment.as_deref(),
        }
    }
}

/// Strict metadata-free document appended to an existing terminology base.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AppendTermbaseInstr {
    /// Portable entries; an empty batch is a successful no-op.
    pub terms: Vec<AppendTermbaseTermInstr>,
}
