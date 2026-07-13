use poprako_orchestra::Oper;

use crate::model::comic::{ComicCoverReservation, ComicEntry, ComicInfo, ComicInfoListSpec, ComicInfoUpdate};
use crate::value::comic::ComicInclOpt;

pub struct CreateComic<'a> {
    pub entry: &'a ComicEntry,
}

impl<'a> Oper for CreateComic<'a> {
    type Output = ComicInfo;
}

pub struct GetComicInfo<'a, 'b> {
    pub id: &'a str,
    pub incls: &'b [ComicInclOpt],
}

impl<'a, 'b> Oper for GetComicInfo<'a, 'b> {
    type Output = ComicInfo;
}

pub struct ListComicInfos<'a> {
    pub spec: &'a ComicInfoListSpec,
}

impl<'a> Oper for ListComicInfos<'a> {
    type Output = Vec<ComicInfo>;
}

pub struct GetComicInfoExcluded<'a, 'b> {
    pub id: &'a str,
    pub incls: &'b [ComicInclOpt],
}

impl<'a, 'b> Oper for GetComicInfoExcluded<'a, 'b> {
    type Output = ComicInfo;
}

pub struct ListComicInfosExcluded<'a> {
    pub spec: &'a ComicInfoListSpec,
}

impl<'a> Oper for ListComicInfosExcluded<'a> {
    type Output = Vec<ComicInfo>;
}

pub struct UpdateComic<'a> {
    pub update: &'a ComicInfoUpdate,
}

impl<'a> Oper for UpdateComic<'a> {
    type Output = ();
}

pub struct ReserveComicCover<'a> {
    pub id: &'a str,
    pub file_extension: &'a str,
}

impl<'a> Oper for ReserveComicCover<'a> {
    type Output = ComicCoverReservation;
}

pub struct MarkComicCoverUploaded<'a> {
    pub id: &'a str,
    pub cover_version: u32,
}

impl<'a> Oper for MarkComicCoverUploaded<'a> {
    type Output = ();
}

pub struct DeleteComic<'a> {
    pub id: &'a str,
}

impl<'a> Oper for DeleteComic<'a> {
    type Output = ();
}

pub struct AllocateComicChapterIndex<'a> {
    pub id: &'a str,
}

impl<'a> Oper for AllocateComicChapterIndex<'a> {
    type Output = i32;
}

pub struct UpdateComicChapterCount<'a> {
    pub id: &'a str,
    pub delta: i32,
}

impl<'a> Oper for UpdateComicChapterCount<'a> {
    type Output = ();
}

pub struct TouchComicLastActive<'a> {
    pub id: &'a str,
}

impl<'a> Oper for TouchComicLastActive<'a> {
    type Output = ();
}
