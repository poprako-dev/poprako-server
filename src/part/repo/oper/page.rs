use poprako_orchestra::Oper;

use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitCountMetrics;
use crate::model::write::page::{PageEntry, PageManifestEntry};

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

/// Lists Page IDs containing visible proofread text diffs.
#[derive(Oper)]
#[oper(output = Vec<String>)]
pub struct ListEdittedDiffPageIds<'a> {
    /// Chapter whose Pages should be checked.
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

/// Applies the complete final page manifest in one typed batch upsert.
#[derive(Oper)]
#[oper(output = Vec<PageInfo>)]
pub struct ApplyPageManifest<'a> {
    /// Final manifest entries in request order.
    pub entries: &'a [PageManifestEntry],
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
