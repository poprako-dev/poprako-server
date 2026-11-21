//! Instr DTOs for the chapter port domain.
//! Data transfer objects for chapter import/export port use cases.
use crate::value::artwork::ArtworkHash;
use crate::value::chapter_port::{
    ChapterTranslationImportMode, ChapterTranslationImportTarget,
    TranslationFormat,
};

/// Translation format used by chapter translation HTTP inputs.
#[derive(Debug, Clone, Copy)]
pub enum ChapterTranslationFormatInstr {
    /// `LabelPlus` translation format.
    LabelPlus,

    /// `Poprako` native translation format.
    Poprako,
}

impl From<ChapterTranslationFormatInstr> for TranslationFormat {
    // Converts the transport format into the domain value.
    fn from(format: ChapterTranslationFormatInstr) -> Self {
        //
        match format {
            //
            ChapterTranslationFormatInstr::LabelPlus => Self::LabelPlus,

            ChapterTranslationFormatInstr::Poprako => Self::PopRaKo,
        }
    }
}

impl From<TranslationFormat> for ChapterTranslationFormatInstr {
    // Converts the domain value into the transport format.
    fn from(format: TranslationFormat) -> Self {
        //
        match format {
            //
            TranslationFormat::LabelPlus => Self::LabelPlus,

            TranslationFormat::PopRaKo => Self::Poprako,
        }
    }
}

/// source page-content strategy selected by a chapter translation import request.
#[derive(Debug, Clone, Copy)]
pub enum ChapterTranslationImportModeInstr {
    /// Preserve pages that already contain visible Units.
    Keep,

    /// Replace every page with the Units represented by the imported source.
    Overwrite,
}

impl From<ChapterTranslationImportModeInstr> for ChapterTranslationImportMode {
    // Converts the transport import strategy into the domain value.
    fn from(mode: ChapterTranslationImportModeInstr) -> Self {
        //
        match mode {
            //
            ChapterTranslationImportModeInstr::Keep => Self::Keep,

            ChapterTranslationImportModeInstr::Overwrite => Self::Overwrite,
        }
    }
}

impl From<ChapterTranslationImportMode> for ChapterTranslationImportModeInstr {
    // Converts the domain import strategy into the transport value.
    fn from(mode: ChapterTranslationImportMode) -> Self {
        //
        match mode {
            //
            ChapterTranslationImportMode::Keep => Self::Keep,

            ChapterTranslationImportMode::Overwrite => Self::Overwrite,
        }
    }
}

/// Text fields selected by a chapter translation import request.
#[derive(Debug, Clone, Copy)]
pub enum ChapterTranslationImportTargetInstr {
    /// Write translated text only.
    Translation,

    /// Write proofread text only.
    Proofread,

    /// Write both translation and proofread text.
    Both,
}

impl From<ChapterTranslationImportTargetInstr>
    for ChapterTranslationImportTarget
{
    // Converts the transport import target into the domain value.
    fn from(target: ChapterTranslationImportTargetInstr) -> Self {
        //
        match target {
            //
            ChapterTranslationImportTargetInstr::Translation => {
                Self::Translation
            }

            ChapterTranslationImportTargetInstr::Proofread => Self::Proofread,

            ChapterTranslationImportTargetInstr::Both => Self::Both,
        }
    }
}

impl From<ChapterTranslationImportTarget>
    for ChapterTranslationImportTargetInstr
{
    // Converts the domain import target into the transport value.
    fn from(target: ChapterTranslationImportTarget) -> Self {
        //
        match target {
            //
            ChapterTranslationImportTarget::Translation => Self::Translation,

            ChapterTranslationImportTarget::Proofread => Self::Proofread,

            ChapterTranslationImportTarget::Both => Self::Both,
        }
    }
}

/// Request body for importing chapter translations.
pub struct ImportChapterTranslationInstr {
    /// The translation format (e.g., JSON, SRT).
    pub format: ChapterTranslationFormatInstr,

    /// source page-content strategy used while applying the imported source.
    pub mode: ChapterTranslationImportModeInstr,

    /// Text fields written by this import.
    pub target: ChapterTranslationImportTargetInstr,

    /// Raw translation content string.
    pub content: String,
}

/// Request to allocate the chapter's single artwork file.
#[derive(Debug)]
pub struct AllocChapterArtworkInstr {
    /// Canonical Base64 SHA-256 identity of the exact uploaded bytes.
    pub artwork_hash: ArtworkHash,
    /// Exact upload length, also bound into the PUT signature.
    pub new_byte_len: u64,
    /// File extension without a leading dot.
    pub ext: String,
}

/// Confirmation of one exact current artwork generation.
#[derive(Debug)]
pub struct MarkChapterArtworkUploadedInstr {
    /// Version returned by allocation, including deduplicated allocation.
    pub artwork_ver: u32,
}
