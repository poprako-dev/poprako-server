use poprako_orchestra::Oper;

use crate::model::read::proj::page_source::{
    PageSourceInfo, PageSourceRawIdentInfo, PageSourceUnitDiffStats,
    PageSourceUnitFlaggedStats, PageSourceUnitScope,
};
use crate::model::read::proj::unit::UnitCountMetrics;
use crate::model::write::page_source::PageSourceManifestEntry;
use crate::shape::write::page_source::PageSourceRawIdentsReplShape;

/// Retrieves a single page's info by ID.
#[derive(Oper)]
#[oper(output = PageSourceInfo)]
pub struct GetPageSourceInfo<'a> {
    /// The page ID.
    pub id: &'a str,
}

/// Retrieves the minimal source page scope needed by Unit operations.
#[derive(Oper)]
#[oper(output = PageSourceUnitScope)]
pub struct GetPageSourceUnitScope<'a> {
    /// The source page ID.
    pub id: &'a str,
}

/// Locks and retrieves the minimal source page scope needed by Unit edits.
#[derive(Oper)]
#[oper(output = PageSourceUnitScope)]
pub struct GetPageSourceUnitScopeExcluded<'a> {
    /// The source page ID.
    pub id: &'a str,
}

/// Lists all pages for a chapter.
#[derive(Oper)]
#[oper(output = Vec<PageSourceInfo>)]
pub struct ListPageSourceInfos<'a> {
    /// The chapter ID.
    pub chapter_id: &'a str,
}

/// Lists Unit text statistics for source pages containing visible revision differences.
#[derive(Oper)]
#[oper(output = Vec<PageSourceUnitDiffStats>)]
pub struct ListPageSourceUnitDiffStats<'a> {
    /// Chapter whose source pages should be checked.
    pub chapter_id: &'a str,
}

/// Finds the lowest-index page for each requested chapter.
#[derive(Oper)]
#[oper(output = Vec<PageSourceInfo>)]
pub struct ListFirstPageSourceInfos<'a> {
    /// The chapter IDs to query.
    pub chapter_ids: &'a [&'a str],
}

/// Retrieves a single page's info by ID with excluded fields omitted.
#[derive(Oper)]
#[oper(output = PageSourceInfo)]
pub struct GetPageSourceInfoExcluded<'a> {
    /// The page ID.
    pub id: &'a str,
}

/// Lists all chapter pages in stable order while holding row locks.
#[derive(Oper)]
#[oper(output = Vec<PageSourceInfo>)]
pub struct ListPageSourceInfosExcluded<'a> {
    /// The chapter ID.
    pub chapter_id: &'a str,
}

/// Moves normal indexes into the transaction-local negative range.
#[derive(Oper)]
#[oper(output = ())]
pub struct ShiftPageSourceIndexesTemporary<'a> {
    /// The chapter ID.
    pub chapter_id: &'a str,
}

/// Applies the complete final page manifest in one typed batch upsert.
#[derive(Oper)]
#[oper(output = Vec<PageSourceInfo>)]
pub struct ApplyPageSourceManifest<'a> {
    /// Final manifest entries in request order.
    pub entries: &'a [PageSourceManifestEntry],
}

/// Sets the unit counters for a page.
#[derive(Oper)]
#[oper(output = ())]
pub struct SetPageSourceUnitCountMetrics<'a> {
    /// The page ID.
    pub id: &'a str,
    /// The unit counters to set.
    pub count_metrics: UnitCountMetrics,
}

/// Deletes pages by chapter or by a list of IDs.
#[derive(Oper)]
#[oper(output = ())]
pub enum DeletePageSources<'a> {
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

/// Reads source filename records for the selected pages.
#[derive(Oper)]
#[oper(output = Vec<PageSourceRawIdentInfo>)]
pub struct ListPageSourceRawIdentInfos<'a> {
    /// source page identifiers included in the export.
    pub page_source_ids: &'a [&'a str],
}

/// Applies complete source filenames within the caller's transaction.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpdatePageSourceRawIdents<'a> {
    /// Paired filenames with distinct resolved page identifiers.
    pub repl: &'a PageSourceRawIdentsReplShape<'a>,
}

/// Lists visible flagged Unit counts for matching source pages in a Chapter.
#[derive(Oper)]
#[oper(output = Vec<PageSourceUnitFlaggedStats>)]
pub struct ListPageSourceUnitFlaggedStats<'a> {
    /// Chapter whose source pages should be checked.
    pub chapter_id: &'a str,
}
