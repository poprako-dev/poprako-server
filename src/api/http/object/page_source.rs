//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::api::http::object::upload::ImageUploadSlotObj;
use crate::api::http::result::Wire;
use crate::data::instr::page_source::{
    AllocChapterPageSourcesInstr, AllocPageSourceImageInstr,
    ListPageSourceInfosInstr, ListPageSourceUnitDiffStatsInstr,
    ListPageSourceUnitFlaggedStatsInstr, MarkPageSourceImageUploadedInstr,
    PageSourceImageInstr,
};
use crate::data::val::page_source::{
    AllocChapterPageSourcesVal, AllocatedPageSourceVal,
    PageSourceUnitDiffStatsVal, PageSourceUnitFlaggedStatsVal,
};
use crate::data::view::page_source::PageSourceInfoView;
use crate::value::image::{ImageExt, ImageHash};

/// Input parameters for reserving all page images of a chapter.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocChapterPageSourcesBody {
    /// Target chapter identifier.
    pub chapter_id: String,
    /// source page images to allocate for the chapter.
    pub pages: Vec<PageSourceImageBody>,
}

impl From<AllocChapterPageSourcesBody> for AllocChapterPageSourcesInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AllocChapterPageSourcesBody) -> Self {
        Self {
            chapter_id: value.chapter_id,
            pages: value
                .pages
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

/// One page image in a complete chapter manifest.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageSourceImageBody {
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

impl From<PageSourceImageBody> for PageSourceImageInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: PageSourceImageBody) -> Self {
        Self {
            raw_ident: value.raw_ident,
            page_source_id: value.page_source_id,
            image_hash: value.image_hash,
            new_byte_len: value.new_byte_len,
            ext: value.ext,
        }
    }
}

/// Input parameters for reserving one page image.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocPageSourceImageBody {
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

impl From<AllocPageSourceImageBody> for AllocPageSourceImageInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AllocPageSourceImageBody) -> Self {
        Self {
            raw_ident: value.raw_ident,
            image_hash: value.image_hash,
            new_byte_len: value.new_byte_len,
            ext: value.ext,
        }
    }
}

/// Input parameters for confirming a page image upload completed.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkPageSourceImageUploadedBody {
    /// Image version from the allocation, used as an idempotency guard.
    #[serde(rename = "image_version")]
    pub image_ver: u32,
}

impl From<MarkPageSourceImageUploadedBody>
    for MarkPageSourceImageUploadedInstr
{
    // Converts the value between the use-case and wire shapes.
    fn from(value: MarkPageSourceImageUploadedBody) -> Self {
        Self {
            image_ver: value.image_ver,
        }
    }
}

/// Input parameters for listing all pages under one chapter.
#[derive(Debug)]
pub struct ListPageSourceInfosBody {
    /// Chapter whose pages to list.
    pub chapter_id: String,
}

impl From<ListPageSourceInfosBody> for ListPageSourceInfosInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListPageSourceInfosBody) -> Self {
        Self {
            chapter_id: value.chapter_id,
        }
    }
}

/// Input parameters for listing source page Unit text statistics.
#[derive(Debug)]
pub struct ListPageSourceUnitDiffStatsBody {
    /// Chapter whose source pages are checked for Unit text diffs.
    pub chapter_id: String,
}

impl From<ListPageSourceUnitDiffStatsBody>
    for ListPageSourceUnitDiffStatsInstr
{
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListPageSourceUnitDiffStatsBody) -> Self {
        Self {
            chapter_id: value.chapter_id,
        }
    }
}

/// Input parameters for listing source pages containing flagged Units.
#[derive(Debug)]
pub struct ListPageSourceUnitFlaggedStatsBody {
    /// Chapter whose source pages are checked for flagged Units.
    pub chapter_id: String,
}

impl From<ListPageSourceUnitFlaggedStatsBody>
    for ListPageSourceUnitFlaggedStatsInstr
{
    // Converts the value between the use-case and wire shapes.
    fn from(value: ListPageSourceUnitFlaggedStatsBody) -> Self {
        Self {
            chapter_id: value.chapter_id,
        }
    }
}

/// Visible Unit text statistics for a source page with revision differences.
/// Counts are independent of revision approval and cover all visible Units.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageSourceUnitDiffStatsPayload {
    /// Permanent source page identifier.
    pub page_source_id: String,
    /// Original zero-based Chapter position, preserved after filtering.
    pub index: usize,

    /// Units with translation text or typesetting annotations; revision-only Units are excluded.
    pub translated_unit_count: usize,
    /// Units with non-whitespace translation and revision texts that differ exactly.
    pub editted_unit_count: usize,
    /// Units with non-whitespace revision text and absent or whitespace-only translation.
    pub proofreader_append_unit_count: usize,
}

impl From<PageSourceUnitDiffStatsVal> for PageSourceUnitDiffStatsPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: PageSourceUnitDiffStatsVal) -> Self {
        Self {
            page_source_id: value.page_source_id,
            index: value.index,
            translated_unit_count: value.translated_unit_count,
            editted_unit_count: value.editted_unit_count,
            proofreader_append_unit_count: value.proofreader_append_unit_count,
        }
    }
}

impl Wire for PageSourceUnitDiffStatsVal {
    // Serializable wire payload produced by this conversion.
    type Target = PageSourceUnitDiffStatsPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Return value from successful chapter page allocations.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocChapterPageSourcesPayload {
    /// Allocated pages with upload targets.
    pub pages: Vec<AllocatedPageSourcePayload>,
}

impl From<AllocChapterPageSourcesVal> for AllocChapterPageSourcesPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AllocChapterPageSourcesVal) -> Self {
        Self {
            pages: value
                .pages
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

impl Wire for AllocChapterPageSourcesVal {
    // Serializable wire payload produced by this conversion.
    type Target = AllocChapterPageSourcesPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// One allocated page upload target.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocatedPageSourcePayload {
    /// Allocated page identifier.
    pub page_source_id: String,

    /// Ordinal position within the chapter.
    pub index: u32,

    /// Content hash of the page image.
    pub image_hash: ImageHash,
    /// File format.
    pub ext: ImageExt,

    /// Current allocated or reused image generation.
    #[serde(rename = "image_version")]
    pub image_ver: u32,

    /// Presigned upload slot, if a new image must be uploaded.
    pub slot: Option<ImageUploadSlotObj>,
}

impl From<AllocatedPageSourceVal> for AllocatedPageSourcePayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AllocatedPageSourceVal) -> Self {
        Self {
            page_source_id: value.page_source_id,
            index: value.index,
            image_hash: value.image_hash,
            ext: value.ext,
            image_ver: value.image_ver,
            slot: value.slot.map(std::convert::Into::into),
        }
    }
}

impl Wire for AllocatedPageSourceVal {
    // Serializable wire payload produced by this conversion.
    type Target = AllocatedPageSourcePayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Visible flagged Unit statistics for one matching source page.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageSourceUnitFlaggedStatsPayload {
    /// Permanent source page identifier.
    pub page_source_id: String,
    /// Original zero-based Chapter position, preserved after filtering.
    pub index: usize,

    /// Number of visible Units flagged for later review.
    pub flagged_unit_count: usize,
}

impl From<PageSourceUnitFlaggedStatsVal> for PageSourceUnitFlaggedStatsPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: PageSourceUnitFlaggedStatsVal) -> Self {
        Self {
            page_source_id: value.page_source_id,
            index: value.index,
            flagged_unit_count: value.flagged_unit_count,
        }
    }
}

impl Wire for PageSourceUnitFlaggedStatsVal {
    // Serializable wire payload produced by this conversion.
    type Target = PageSourceUnitFlaggedStatsPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Presentation-ready page information.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageSourceInfoObj {
    /// Unique page identifier.
    pub id: String,

    /// Owning chapter identifier.
    pub chapter_id: String,

    /// Ordinal position within the chapter.
    pub index: usize,

    /// Complete original source filename, when supplied.
    pub raw_ident: Option<String>,

    /// Current allocated image generation.
    #[serde(rename = "image_version")]
    pub image_ver: Option<u32>,
    /// Whether the current generation is available to readers.
    pub image_uploaded: bool,

    /// Presigned download URL for the full image, if uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// Download URL for the optimized image, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_optimized_url: Option<String>,
    /// Presigned download URL for the thumbnail image, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_thumbnail_url: Option<String>,
    /// Content hash of the page image, if one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_hash: Option<ImageHash>,
    /// File format, if one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<ImageExt>,

    /// Total number of translation units on this page.
    pub total_unit_count: usize,
    /// Number of translated units on this page.
    pub translated_unit_count: usize,
    /// Number of proofread units on this page.
    pub proofread_unit_count: usize,

    /// Timestamp of creation, in Unix milliseconds.
    pub created_at: i64,
    /// Timestamp of last update, in Unix milliseconds.
    pub updated_at: i64,
}

impl From<PageSourceInfoView> for PageSourceInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: PageSourceInfoView) -> Self {
        Self {
            id: value.id,
            chapter_id: value.chapter_id,
            index: value.index,
            raw_ident: value.raw_ident,
            image_ver: value.image_ver,
            image_uploaded: value.image_uploaded,
            image_url: value.image_url,
            image_optimized_url: value.image_optimized_url,
            image_thumbnail_url: value.image_thumbnail_url,
            image_hash: value.image_hash,
            ext: value.ext,
            total_unit_count: value.total_unit_count,
            translated_unit_count: value.translated_unit_count,
            proofread_unit_count: value.proofread_unit_count,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl Wire for PageSourceInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = PageSourceInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}
