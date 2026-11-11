//! Instr DTOs for the comic archive domain.
//! Request and response values for immutable comic archive operations.
/// Query parameters for exporting selected archive month slots.
#[derive(Debug)]
pub struct ExportComicArchivesInstr {
    /// UTC month slots in `YYYY-MM` format to filter archives by.
    pub month: Vec<String>,
}
