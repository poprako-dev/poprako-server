//! Instr DTOs for the page domain.

//! Data transfer objects for page use cases.

use serde::Deserialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::model::write::page_source::PageSourceImageSpec;
use crate::value::image::{ImageExt, ImageHash};

/// Input parameters for reserving all page images of a chapter.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocChapterPageSourcesInstr {
    /// Target chapter identifier.
    pub chapter_id: String,
    /// source page images to allocate for the chapter.
    pub pages: Vec<PageSourceImageInstr>,
}

/// One page image in a complete chapter manifest.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageSourceImageInstr {
    /// Complete original filename. Omission or null replaces it with no name.
    #[serde(default)]
    pub raw_ident: Option<String>,

    //
    /// Existing page identifier, if updating an existing page.
    pub page_source_id: Option<String>,
    /// Content hash of the page image.
    pub image_hash: ImageHash,
    /// Size used to constrain a newly requested upload slot.
    ///
    /// Existing manifest entries may omit this when no upload slot is needed.
    pub new_byte_len: Option<u64>,
    /// File format.
    pub ext: ImageExt,
}

impl From<PageSourceImageInstr> for PageSourceImageSpec {
    // Map page image parameters directly to the domain spec.
    fn from(instr: PageSourceImageInstr) -> Self {
        Self {
            raw_ident: instr.raw_ident,
            page_source_id: instr.page_source_id,
            image_hash: instr.image_hash,
            new_byte_len: instr.new_byte_len,
            ext: instr.ext,
        }
    }
}

/// Input parameters for reserving one page image.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocPageSourceImageInstr {
    /// Complete original filename. Omission or null replaces it with no name.
    #[serde(default)]
    pub raw_ident: Option<String>,

    //
    /// Content hash of the page image to allocate.
    pub image_hash: ImageHash,
    /// Size of the page image in bytes.
    pub new_byte_len: u64,
    /// File format.
    pub ext: ImageExt,
}

/// Input parameters for confirming a page image upload completed.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkPageSourceImageUploadedInstr {
    /// Image version from the allocation, used as an idempotency guard.
    #[serde(rename = "image_version")]
    pub image_ver: u32,
}

/// Input parameters for listing all pages under one chapter.
#[derive(Debug)]
pub struct ListPageSourceInfosInstr {
    /// Chapter whose pages to list.
    pub chapter_id: String,
}

/// Input parameters for listing source page Unit text statistics.
#[derive(Debug)]
pub struct ListPageSourceUnitDiffStatsInstr {
    /// Chapter whose source pages are checked for Unit text diffs.
    pub chapter_id: String,
}

/// Input parameters for listing source pages containing flagged Units.
#[derive(Debug)]
pub struct ListPageSourceUnitFlaggedStatsInstr {
    /// Chapter whose source pages are checked for flagged Units.
    pub chapter_id: String,
}
