//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::api::http::object::upload::ImageUploadSlotObj;
use crate::api::http::result::Wire;
use crate::data::instr::page_artwork::{
    AllocChapterPageArtworksInstr, AllocPageArtworkImageInstr,
    MarkPageArtworkImageUploadedInstr, PageArtworkImageInstr,
};
use crate::data::val::page_artwork::{
    AllocChapterPageArtworksVal, AllocatedPageArtworkVal,
};
use crate::data::view::page_artwork::PageArtworkInfoView;
use crate::value::image::{ImageExt, ImageHash};

/// One item in an authoritative composite manifest.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageArtworkImageBody {
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

impl From<PageArtworkImageBody> for PageArtworkImageInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: PageArtworkImageBody) -> Self {
        Self {
            page_artwork_id: value.page_artwork_id,
            raw_ident: value.raw_ident,
            image_hash: value.image_hash,
            new_byte_len: value.new_byte_len,
            ext: value.ext,
        }
    }
}

/// Complete independently ordered composite page manifest.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocChapterPageArtworksBody {
    /// Final page sequence; omitted old pages are deleted.
    pub pages: Vec<PageArtworkImageBody>,
}

impl From<AllocChapterPageArtworksBody> for AllocChapterPageArtworksInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AllocChapterPageArtworksBody) -> Self {
        Self {
            pages: value
                .pages
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

/// Single composite image replacement.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocPageArtworkImageBody {
    /// Original PSD filename or relative path.
    pub raw_ident: Option<String>,
    /// Composite image hash.
    pub image_hash: ImageHash,
    /// Exact upload byte length.
    pub new_byte_len: u64,
    /// Composite image format.
    pub ext: ImageExt,
}

impl From<AllocPageArtworkImageBody> for AllocPageArtworkImageInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AllocPageArtworkImageBody) -> Self {
        Self {
            raw_ident: value.raw_ident,
            image_hash: value.image_hash,
            new_byte_len: value.new_byte_len,
            ext: value.ext,
        }
    }
}

/// Exact-generation composite upload confirmation.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkPageArtworkImageUploadedBody {
    /// Allocated image version.
    #[serde(rename = "image_version")]
    pub image_ver: u32,
}

impl From<MarkPageArtworkImageUploadedBody>
    for MarkPageArtworkImageUploadedInstr
{
    // Converts the value between the use-case and wire shapes.
    fn from(value: MarkPageArtworkImageUploadedBody) -> Self {
        Self {
            image_ver: value.image_ver,
        }
    }
}

/// One resolved composite page and its current image generation.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocatedPageArtworkPayload {
    /// Stable composite page identity.
    pub page_artwork_id: String,

    /// Final page position.
    pub index: usize,

    /// Composite image hash.
    pub image_hash: ImageHash,
    /// Image format.
    pub ext: ImageExt,
    /// Allocated or reused image generation.
    #[serde(rename = "image_version")]
    pub image_ver: u32,

    /// Upload capability; absent when the exact content is already available.
    pub slot: Option<ImageUploadSlotObj>,
}

impl From<AllocatedPageArtworkVal> for AllocatedPageArtworkPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AllocatedPageArtworkVal) -> Self {
        Self {
            page_artwork_id: value.page_artwork_id,
            index: value.index,
            image_hash: value.image_hash,
            ext: value.ext,
            image_ver: value.image_ver,
            slot: value.slot.map(std::convert::Into::into),
        }
    }
}

impl Wire for AllocatedPageArtworkVal {
    // Serializable wire payload produced by this conversion.
    type Target = AllocatedPageArtworkPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Allocations aligned with the complete submitted manifest.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct AllocChapterPageArtworksPayload {
    /// Resolved pages in request order.
    pub pages: Vec<AllocatedPageArtworkPayload>,
}

impl From<AllocChapterPageArtworksVal> for AllocChapterPageArtworksPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: AllocChapterPageArtworksVal) -> Self {
        Self {
            pages: value
                .pages
                .into_iter()
                .map(std::convert::Into::into)
                .collect(),
        }
    }
}

impl Wire for AllocChapterPageArtworksVal {
    // Serializable wire payload produced by this conversion.
    type Target = AllocChapterPageArtworksPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Presentation-ready composite artwork page information.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct PageArtworkInfoObj {
    /// Unique page identifier.
    pub id: String,

    /// Owning chapter identifier.
    pub chapter_id: String,
    /// Ordinal position within the chapter.
    pub index: usize,

    /// Original PSD filename or relative path.
    pub raw_ident: Option<String>,

    /// Presigned download URL for the full image, if uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// Presigned download URL for the thumbnail image, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_thumbnail_url: Option<String>,
    /// Content hash of the page image, if one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_hash: Option<ImageHash>,
    /// File format, if one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ext: Option<ImageExt>,
    /// Current allocated image version.
    #[serde(rename = "image_version")]
    pub image_ver: Option<u32>,
    /// Whether the current image is available.
    pub image_uploaded: bool,

    /// Timestamp of creation, in Unix milliseconds.
    pub created_at: i64,
    /// Timestamp of last update, in Unix milliseconds.
    pub updated_at: i64,
}

impl From<PageArtworkInfoView> for PageArtworkInfoObj {
    // Converts the value between the use-case and wire shapes.
    fn from(value: PageArtworkInfoView) -> Self {
        Self {
            id: value.id,
            chapter_id: value.chapter_id,
            index: value.index,
            raw_ident: value.raw_ident,
            image_url: value.image_url,
            image_thumbnail_url: value.image_thumbnail_url,
            image_hash: value.image_hash,
            ext: value.ext,
            image_ver: value.image_ver,
            image_uploaded: value.image_uploaded,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl Wire for PageArtworkInfoView {
    // Serializable wire payload produced by this conversion.
    type Target = PageArtworkInfoObj;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}
