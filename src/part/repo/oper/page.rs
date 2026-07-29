use std::collections::HashMap;

use poprako_orchestra::Oper;

use crate::model::page::{
    PageEntry, PageImageReservation, PageInfo, PageManifestUpdate,
};
use crate::model::read::proj::unit::UnitCounters;

/// Retrieves a single page's info by ID.
pub struct GetPageInfo<'a> {
    /// The page ID.
    pub id: &'a str,
}

impl Oper for GetPageInfo<'_> {
    // The retrieved page info.
    type Output = PageInfo;
}

/// Lists all pages for a chapter.
pub struct ListPageInfos<'a> {
    /// The chapter ID.
    pub chapter_id: &'a str,
}

impl Oper for ListPageInfos<'_> {
    // List of matching page infos.
    type Output = Vec<PageInfo>;
}

/// Finds the lowest-index page for each requested chapter.
pub struct ListFirstPageInfos<'a> {
    /// The chapter IDs to query.
    pub chapter_ids: &'a [String],
}

impl Oper for ListFirstPageInfos<'_> {
    // Map of chapter ID to its first page info.
    type Output = HashMap<String, PageInfo>;
}

/// Creates multiple pages from the given entries.
pub struct CreatePages<'a> {
    /// The page entries to insert.
    pub entries: &'a [PageEntry],
}

impl Oper for CreatePages<'_> {
    // The created page infos.
    type Output = Vec<PageInfo>;
}

/// Retrieves a single page's info by ID with excluded fields omitted.
pub struct GetPageInfoExcluded<'a> {
    /// The page ID.
    pub id: &'a str,
}

impl Oper for GetPageInfoExcluded<'_> {
    // The retrieved page info with excluded fields omitted.
    type Output = PageInfo;
}

/// Lists all chapter pages in stable order while holding row locks.
pub struct ListPageInfosExcluded<'a> {
    /// The chapter ID.
    pub chapter_id: &'a str,
}

impl Oper for ListPageInfosExcluded<'_> {
    // List of page infos with excluded fields omitted.
    type Output = Vec<PageInfo>;
}

/// Moves normal indexes into the transaction-local negative range.
pub struct ShiftPageIndexesTemporary<'a> {
    /// The chapter ID.
    pub chapter_id: &'a str,
}

impl Oper for ShiftPageIndexesTemporary<'_> {
    // Unit on success.
    type Output = ();
}

/// Updates one retained page to its final manifest identity and position.
pub struct UpdatePageManifest<'a> {
    /// The manifest update payload.
    pub update: &'a PageManifestUpdate,
}

impl Oper for UpdatePageManifest<'_> {
    // The updated page info.
    type Output = PageInfo;
}

/// Invalidates all page image keys after chapter publication.
pub struct ClearPageImagesForPublish<'a> {
    /// The chapter ID whose images to clear.
    pub chapter_id: &'a str,
}

impl Oper for ClearPageImagesForPublish<'_> {
    // List of storage keys for invalidated images.
    type Output = Vec<String>;
}

/// Reserves an image slot for a page.
pub struct ReservePageImage<'a> {
    //
    /// The page ID.
    pub id: &'a str,
    /// The file extension for the image.
    pub file_ext: &'a str,
}

impl Oper for ReservePageImage<'_> {
    // The image reservation details.
    type Output = PageImageReservation;
}

/// Marks a page image as uploaded.
pub struct MarkPageImageUploaded<'a> {
    //
    /// The page ID.
    pub id: &'a str,
    /// The image version to mark.
    pub image_version: u32,
    /// The optional S3 key for the uploaded image.
    pub image_key: Option<&'a str>,
}

impl Oper for MarkPageImageUploaded<'_> {
    // Unit on success.
    type Output = ();
}

/// Sets one page image's verified upload state for its current identity.
pub struct SetPageImageUploaded<'a> {
    //
    /// The page ID.
    pub id: &'a str,
    /// The image version to update.
    pub image_version: u32,
    /// The S3 key for the image.
    pub image_key: &'a str,
    /// Whether the image is uploaded.
    pub image_uploaded: bool,
}

impl Oper for SetPageImageUploaded<'_> {
    // Unit on success.
    type Output = ();
}

/// Sets the unit counters for a page.
pub struct SetPageUnitCounters<'a> {
    //
    /// The page ID.
    pub id: &'a str,
    /// The unit counters to set.
    pub counters: UnitCounters,
}

impl Oper for SetPageUnitCounters<'_> {
    // Unit on success.
    type Output = ();
}

/// Deletes pages by chapter or by a list of IDs.
pub enum DeletePages<'a> {
    /// Deletes all pages for a chapter.
    Chapter {
        /// The chapter ID.
        chapter_id: &'a str,
    },

    /// Deletes specific pages by ID.
    Ids {
        /// The page IDs to delete.
        ids: &'a [String],
    },
}

impl Oper for DeletePages<'_> {
    // Unit on success.
    type Output = ();
}
