use std::collections::HashMap;

use poprako_orchestra::Oper;

use crate::model::page::{
    PageEntry, PageImageReservation, PageInfo, PageManifestUpdate,
};
use crate::model::unit::UnitCounters;

pub struct GetPageInfo<'a> {
    pub id: &'a str,
}

impl<'a> Oper for GetPageInfo<'a> {
    type Output = PageInfo;
}

pub enum ListPageInfos<'a> {
    Chapter {
        chapter_id: &'a str,
        offset: u32,
        limit: u32,
    },
    AllChapter {
        chapter_id: &'a str,
    },
}

impl<'a> Oper for ListPageInfos<'a> {
    type Output = Vec<PageInfo>;
}

/// Finds the lowest-index page for each requested chapter.
pub struct ListFirstPageInfos<'a> {
    pub chapter_ids: &'a [String],
}

impl<'a> Oper for ListFirstPageInfos<'a> {
    type Output = HashMap<String, PageInfo>;
}

pub struct CreatePages<'a> {
    pub entries: &'a [PageEntry],
}

impl<'a> Oper for CreatePages<'a> {
    type Output = Vec<PageInfo>;
}

pub struct GetPageInfoExcluded<'a> {
    pub id: &'a str,
}

/// Lists all chapter pages in stable order while holding row locks.
pub struct ListPageInfosExcluded<'a> {
    pub chapter_id: &'a str,
}

impl<'a> Oper for ListPageInfosExcluded<'a> {
    type Output = Vec<PageInfo>;
}

/// Moves normal indexes into the transaction-local negative range.
pub struct ShiftPageIndexesTemporary<'a> {
    pub chapter_id: &'a str,
}

impl<'a> Oper for ShiftPageIndexesTemporary<'a> {
    type Output = ();
}

/// Updates one retained page to its final manifest identity and position.
pub struct UpdatePageManifest<'a> {
    pub update: &'a PageManifestUpdate,
}

impl<'a> Oper for UpdatePageManifest<'a> {
    type Output = PageInfo;
}

impl<'a> Oper for GetPageInfoExcluded<'a> {
    type Output = PageInfo;
}

pub struct ReservePageImage<'a> {
    pub id: &'a str,
    pub file_ext: &'a str,
}

impl<'a> Oper for ReservePageImage<'a> {
    type Output = PageImageReservation;
}

pub struct MarkPageImageUploaded<'a> {
    pub id: &'a str,
    pub image_version: u32,
    pub image_key: Option<&'a str>,
}

impl<'a> Oper for MarkPageImageUploaded<'a> {
    type Output = ();
}

pub struct SetPageUnitCounters<'a> {
    pub id: &'a str,
    pub counters: UnitCounters,
}

impl<'a> Oper for SetPageUnitCounters<'a> {
    type Output = ();
}

pub enum DeletePages<'a> {
    Chapter { chapter_id: &'a str },
    Ids { ids: &'a [String] },
}

impl<'a> Oper for DeletePages<'a> {
    type Output = ();
}
