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
