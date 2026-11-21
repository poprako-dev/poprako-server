//! Composite page upload instructions.
use crate::value::image::{ImageExt, ImageHash};

/// One item in an authoritative composite manifest.
pub struct PageArtworkImageInstr {
    /// Existing composite page identity; omission creates a new page.
    pub page_artwork_id: Option<String>,
    /// Original PSD filename or relative path, for presentation only.
    pub raw_ident: Option<String>,

    /// SHA-256 of the extracted composite image.
    pub image_hash: ImageHash,
    /// Required for a new or unavailable image generation.
    pub new_byte_len: Option<u64>,
    /// Composite image format.
    pub ext: ImageExt,
}

/// Complete independently ordered composite page manifest.
pub struct AllocChapterPageArtworksInstr {
    /// Final page sequence; omitted old pages are deleted.
    pub pages: Vec<PageArtworkImageInstr>,
}

/// Single composite image replacement.
#[derive(Debug)]
pub struct AllocPageArtworkImageInstr {
    /// Original PSD filename or relative path.
    pub raw_ident: Option<String>,
    /// Composite image hash.
    pub image_hash: ImageHash,
    /// Exact upload byte length.
    pub new_byte_len: u64,
    /// Composite image format.
    pub ext: ImageExt,
}

/// Exact-generation composite upload confirmation.
#[derive(Debug)]
pub struct MarkPageArtworkImageUploadedInstr {
    /// Allocated image version.
    pub image_ver: u32,
}
