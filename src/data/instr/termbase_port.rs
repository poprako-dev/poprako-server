//! Request DTOs for native terminology-base import.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::model::write::term::TermImport;
use crate::model::write::termbase::TermbaseImport;

/// One portable terminology entry supplied by an import request.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportTermInstr {
    //
    /// Source-language term or phrase.
    pub source: String,
    /// Target-language translations.
    pub targets: Vec<String>,
    /// Optional annotation or usage note.
    pub comment: Option<String>,
}

impl From<ImportTermInstr> for TermImport {
    // Convert one import entry into the domain write model.
    fn from(instr: ImportTermInstr) -> Self {
        //
        Self {
            source: instr.source,
            targets: instr.targets,
            comment: instr.comment,
        }
    }
}

/// Native terminology-base document supplied to an import endpoint.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ImportTermbaseInstr {
    //
    /// Display name of the terminology base.
    pub name: String,
    /// Optional terminology-base description.
    pub description: Option<String>,
    /// Portable terminology entries to import.
    pub terms: Vec<ImportTermInstr>,
}

impl From<ImportTermbaseInstr> for TermbaseImport {
    // Convert the complete import document into the domain write model.
    fn from(instr: ImportTermbaseInstr) -> Self {
        //
        Self {
            name: instr.name,
            description: instr.description,
            terms: instr.terms.into_iter().map(Into::into).collect(),
        }
    }
}
