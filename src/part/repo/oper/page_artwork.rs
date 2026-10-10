//! Composite page repository operations.

use poprako_orchestra::Oper;

use crate::model::read::proj::page_artwork::PageArtworkInfo;
use crate::model::write::page_artwork::{PageArtworkEntry, PageArtworkPatch};

/// Reads a composite page's owning Chapter before coordinated mutation.
#[derive(Oper)]
#[oper(output = PageArtworkInfo)]
pub struct GetPageArtworkInfo<'a> {
    /// Page identity.
    pub id: &'a str,
}

/// Lists a Chapter's composite pages in order.
#[derive(Oper)]
#[oper(output = Vec<PageArtworkInfo>)]
pub struct ListPageArtworkInfos<'a> {
    /// Owning Chapter.
    pub chapter_id: &'a str,
}

/// Replaces the complete composite manifest under the caller's Chapter lock.
#[derive(Oper)]
#[oper(output = ())]
pub struct ReplacePageArtworkManifest<'a> {
    /// Owning Chapter.
    pub chapter_id: &'a str,
    /// Complete final manifest.
    pub entries: &'a [PageArtworkEntry],
}

/// Updates only one composite page's presentation metadata under its Chapter lock.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpdatePageArtworkInfo<'a> {
    /// Metadata replacement for the targeted page.
    pub update: &'a PageArtworkPatch,
}
