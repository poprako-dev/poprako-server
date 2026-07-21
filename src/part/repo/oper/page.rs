use std::collections::HashMap;

use poprako_orchestra::Oper;

use crate::model::page::{
    PageEntry, PageImageReservation, PageInfo, PageManifestUpdate,
};
use crate::model::unit::UnitCounters;

pub struct GetPageInfo<'a> {
    pub id: &'a str,
}

impl Oper for GetPageInfo<'_> {
    type Output = PageInfo;
}

pub struct ListPageInfos<'a> {
    pub chapter_id: &'a str,
}

impl Oper for ListPageInfos<'_> {
    type Output = Vec<PageInfo>;
}

/// Finds the lowest-index page for each requested chapter.
pub struct ListFirstPageInfos<'a> {
    pub chapter_ids: &'a [String],
}

impl Oper for ListFirstPageInfos<'_> {
    type Output = HashMap<String, PageInfo>;
}

pub struct CreatePages<'a> {
    pub entries: &'a [PageEntry],
}

impl Oper for CreatePages<'_> {
    type Output = Vec<PageInfo>;
}

pub struct GetPageInfoExcluded<'a> {
    pub id: &'a str,
}

/// Lists all chapter pages in stable order while holding row locks.
pub struct ListPageInfosExcluded<'a> {
    pub chapter_id: &'a str,
}

impl Oper for ListPageInfosExcluded<'_> {
    type Output = Vec<PageInfo>;
}

/// Moves normal indexes into the transaction-local negative range.
pub struct ShiftPageIndexesTemporary<'a> {
    pub chapter_id: &'a str,
}

impl Oper for ShiftPageIndexesTemporary<'_> {
    type Output = ();
}

/// Updates one retained page to its final manifest identity and position.
pub struct UpdatePageManifest<'a> {
    pub update: &'a PageManifestUpdate,
}

/// Invalidates all page image keys after chapter publication.
pub struct ClearPageImagesForPublish<'a> {
    pub chapter_id: &'a str,
}

impl Oper for ClearPageImagesForPublish<'_> {
    type Output = Vec<String>;
}

impl Oper for UpdatePageManifest<'_> {
    type Output = PageInfo;
}

impl Oper for GetPageInfoExcluded<'_> {
    type Output = PageInfo;
}

pub struct ReservePageImage<'a> {
    pub id: &'a str,
    pub file_ext: &'a str,
}

impl Oper for ReservePageImage<'_> {
    type Output = PageImageReservation;
}

pub struct MarkPageImageUploaded<'a> {
    pub id: &'a str,
    pub image_version: u32,
    pub image_key: Option<&'a str>,
}

impl Oper for MarkPageImageUploaded<'_> {
    type Output = ();
}

/// Sets one page image's verified upload state for its current identity.
pub struct SetPageImageUploaded<'a> {
    pub id: &'a str,
    pub image_version: u32,
    pub image_key: &'a str,
    pub image_uploaded: bool,
}

impl Oper for SetPageImageUploaded<'_> {
    type Output = ();
}

pub struct SetPageUnitCounters<'a> {
    pub id: &'a str,
    pub counters: UnitCounters,
}

impl Oper for SetPageUnitCounters<'_> {
    type Output = ();
}

pub enum DeletePages<'a> {
    Chapter { chapter_id: &'a str },
    Ids { ids: &'a [String] },
}

impl Oper for DeletePages<'_> {
    type Output = ();
}
