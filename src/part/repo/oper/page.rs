use poprako_orchestra::Oper;

use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitCountMetrics;
use crate::model::write::page::{
    PageEntry, PageImageRepl, PageImageReservation, PageManifestRepl,
};

/// Retrieves a single page's info by ID.
#[derive(Oper)]
#[oper(output = PageInfo)]
pub struct GetPageInfo<'a> {
    /// The page ID.
    pub id: &'a str,
}

/// Lists all pages for a chapter.
#[derive(Oper)]
#[oper(output = Vec<PageInfo>)]
pub struct ListPageInfos<'a> {
    /// The chapter ID.
    pub chapter_id: &'a str,
}

/// Finds the lowest-index page for each requested chapter.
#[derive(Oper)]
#[oper(output = Vec<PageInfo>)]
pub struct ListFirstPageInfos<'a> {
    /// The chapter IDs to query.
    pub chapter_ids: &'a [String],
}

/// Creates multiple pages from the given entries.
#[derive(Oper)]
#[oper(output = Vec<PageInfo>)]
pub struct CreatePages<'a> {
    /// The page entries to insert.
    pub entries: &'a [PageEntry],
}

/// Retrieves a single page's info by ID with excluded fields omitted.
#[derive(Oper)]
#[oper(output = PageInfo)]
pub struct GetPageInfoExcluded<'a> {
    /// The page ID.
    pub id: &'a str,
}

/// Lists all chapter pages in stable order while holding row locks.
#[derive(Oper)]
#[oper(output = Vec<PageInfo>)]
pub struct ListPageInfosExcluded<'a> {
    /// The chapter ID.
    pub chapter_id: &'a str,
}

/// Moves normal indexes into the transaction-local negative range.
#[derive(Oper)]
#[oper(output = ())]
pub struct ShiftPageIndexesTemporary<'a> {
    /// The chapter ID.
    pub chapter_id: &'a str,
}

/// Updates one retained page to its final manifest identity and position.
#[derive(Oper)]
#[oper(output = PageInfo)]
pub struct UpdatePageManifest<'a> {
    /// The manifest update payload.
    pub update: &'a PageManifestRepl,
}

/// Invalidates all page image keys after chapter publication.
#[derive(Oper)]
#[oper(output = Vec<String>)]
pub struct ClearPageImagesForPublish<'a> {
    /// The chapter ID whose images to clear.
    pub chapter_id: &'a str,
}

/// Reserves an image slot for a page.
#[derive(Oper)]
#[oper(output = PageImageReservation)]
pub struct ReservePageImage<'a> {
    //
    /// The page ID.
    pub id: &'a str,
    /// The file extension for the image.
    pub file_ext: &'a str,
}

/// Marks a page image as uploaded.
#[derive(Oper)]
#[oper(output = ())]
pub struct MarkPageImageUploaded<'a> {
    /// The replacement payload.
    pub repl: &'a PageImageRepl,
}

/// Sets one page image's verified upload state for its current identity.
#[derive(Oper)]
#[oper(output = ())]
pub struct SetPageImageUploaded<'a> {
    /// The replacement payload.
    pub repl: &'a PageImageRepl,
}

/// Sets the unit counters for a page.
#[derive(Oper)]
#[oper(output = ())]
pub struct SetPageUnitCounters<'a> {
    //
    /// The page ID.
    pub id: &'a str,
    /// The unit counters to set.
    pub counters: UnitCountMetrics,
}

/// Deletes pages by chapter or by a list of IDs.
#[derive(Oper)]
#[oper(output = ())]
pub enum DeletePages<'a> {
    //
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
