//! Instr DTOs for the page domain.

//! Data transfer objects for page use cases.

use serde::Deserialize;

use crate::value::image::{ImageExt, ImageHash};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::model::write::page::PageImageSpec;

/// Input parameters for reserving all page images of a chapter.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReserveChapterPagesInstr {
    //
    /// Target chapter identifier.
    pub chapter_id: String,
    /// Page images to reserve for the chapter.
    pub pages: Vec<PageImageInstr>,
}

/// One page image in a complete chapter manifest.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageImageInstr {
    //
    /// Existing page identifier, if updating an existing page.
    pub page_id: Option<String>,
    /// Content hash of the page image.
    pub image_hash: ImageHash,
    /// Size used to constrain a newly requested upload slot.
    ///
    /// Existing manifest entries may omit this when no upload slot is needed.
    pub new_byte_len: Option<u64>,
    /// File format.
    pub ext: ImageExt,
}

impl From<PageImageInstr> for PageImageSpec {
    // Map page image parameters directly to the domain spec.
    fn from(instr: PageImageInstr) -> Self {
        Self {
            page_id: instr.page_id,
            image_hash: instr.image_hash,
            new_byte_len: instr.new_byte_len,
            ext: instr.ext,
        }
    }
}

/// Input parameters for reserving one page image.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct ReservePageImageInstr {
    //
    /// Content hash of the page image to reserve.
    pub image_hash: ImageHash,
    /// Size of the page image in bytes.
    pub new_byte_len: u64,
    /// File format.
    pub ext: ImageExt,
}

/// Input parameters for confirming a page image upload completed.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkPageImageUploadedInstr {
    /// Image version from the reservation, used as an idempotency guard.
    pub image_version: u32,
}

/// Input parameters for listing all pages under one chapter.
#[derive(Debug)]
pub struct ListPageInfosInstr {
    /// Chapter whose pages to list.
    pub chapter_id: String,
}
