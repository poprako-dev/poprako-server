use poprako_orchestra::Oper;

use crate::model::comic::{
    ComicCoverReservation, ComicEntry, ComicInfo, ComicInfoListSpec,
    ComicInfoUpdate,
};
use crate::value::comic::ComicInclOpt;
use crate::value::image::{ImageExt, ImageHash};

pub struct CreateComic<'a> {
    pub entry: &'a ComicEntry,
}

impl Oper for CreateComic<'_> {
    type Output = ComicInfo;
}

pub struct GetComicInfo<'a, 'b> {
    pub id: &'a str,
    pub incls: &'b [ComicInclOpt],
}

impl Oper for GetComicInfo<'_, '_> {
    type Output = ComicInfo;
}

pub struct ListComicInfos<'a> {
    pub spec: &'a ComicInfoListSpec,
}

impl Oper for ListComicInfos<'_> {
    type Output = Vec<ComicInfo>;
}

pub struct GetComicInfoExcluded<'a, 'b> {
    pub id: &'a str,
    pub incls: &'b [ComicInclOpt],
}

impl Oper for GetComicInfoExcluded<'_, '_> {
    type Output = ComicInfo;
}

pub struct ListComicInfosExcluded<'a> {
    pub spec: &'a ComicInfoListSpec,
}

impl Oper for ListComicInfosExcluded<'_> {
    type Output = Vec<ComicInfo>;
}

pub struct UpdateComic<'a> {
    pub update: &'a ComicInfoUpdate,
}

impl Oper for UpdateComic<'_> {
    type Output = ();
}

pub struct ReserveComicCover<'a> {
    pub id: &'a str,
    pub image_hash: &'a ImageHash,
    pub image_ext: ImageExt,
}

impl Oper for ReserveComicCover<'_> {
    type Output = ComicCoverReservation;
}

pub struct MarkComicCoverUploaded<'a> {
    pub id: &'a str,
    pub cover_version: u32,
    pub cover_key: Option<&'a str>,
    pub cover_uploaded: bool,
}

impl Oper for MarkComicCoverUploaded<'_> {
    type Output = ();
}

pub struct DeleteComic<'a> {
    pub id: &'a str,
}

impl Oper for DeleteComic<'_> {
    type Output = ();
}

pub struct AllocComicChapterIndex<'a> {
    pub id: &'a str,
}

impl Oper for AllocComicChapterIndex<'_> {
    type Output = i32;
}

pub struct UpdateComicChapterCount<'a> {
    pub id: &'a str,
    pub delta: i32,
}

impl Oper for UpdateComicChapterCount<'_> {
    type Output = ();
}

pub struct TouchComicLastActive<'a> {
    pub id: &'a str,
}

impl Oper for TouchComicLastActive<'_> {
    type Output = ();
}
