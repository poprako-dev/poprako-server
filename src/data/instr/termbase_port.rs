//! Request DTOs for native terminology-base import.
use crate::model::write::term::TermImport;
use crate::model::write::termbase::TermbaseImport;

/// One portable terminology entry supplied by an import request.
pub struct ImportTermInstr {
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
        Self {
            source: instr.source,
            targets: instr.targets,
            comment: instr.comment,
        }
    }
}

/// Native terminology-base document supplied to an import endpoint.
pub struct ImportTermbaseInstr {
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
        Self {
            name: instr.name,
            description: instr.description,
            terms: instr.terms.into_iter().map(Into::into).collect(),
        }
    }
}

/// One strict portable terminology entry supplied by an append request.
pub struct AppendTermbaseTermInstr {
    /// Source-language term or phrase.
    pub source: String,
    /// Target-language translations in preferred order.
    pub targets: Vec<String>,
    /// Optional complete annotation block to merge.
    pub comment: Option<String>,
}

impl From<AppendTermbaseTermInstr> for TermImport {
    // Convert append content without introducing terminology-base metadata.
    fn from(instr: AppendTermbaseTermInstr) -> Self {
        Self {
            source: instr.source,
            targets: instr.targets,
            comment: instr.comment,
        }
    }
}

/// Strict metadata-free document appended to an existing terminology base.
pub struct AppendTermbaseInstr {
    /// Portable entries; an empty batch is a successful no-op.
    pub terms: Vec<AppendTermbaseTermInstr>,
}
