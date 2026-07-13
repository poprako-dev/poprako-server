use std::collections::HashMap;

use poprako_orchestra::Oper;

use crate::model::chapter::{
    ChapterEntry, ChapterInfo, ChapterInfoListSpec, ChapterInfoUpdate, ChapterStageUpdate,
};
use crate::model::unit::UnitCounterDelta;
use crate::value::chapter::ChapterInclOpt;

pub struct CreateChapter<'a> {
    pub entry: &'a ChapterEntry,
}

impl<'a> Oper for CreateChapter<'a> {
    type Output = ChapterInfo;
}

pub struct GetChapterInfo<'a, 'b> {
    pub id: &'a str,
    pub incls: &'b [ChapterInclOpt],
}

impl<'a, 'b> Oper for GetChapterInfo<'a, 'b> {
    type Output = ChapterInfo;
}

pub struct GetChapterInfoExcluded<'a, 'b> {
    pub id: &'a str,
    pub incls: &'b [ChapterInclOpt],
}

impl<'a, 'b> Oper for GetChapterInfoExcluded<'a, 'b> {
    type Output = ChapterInfo;
}

pub struct ListChapterInfos<'a> {
    pub spec: &'a ChapterInfoListSpec,
}

impl<'a> Oper for ListChapterInfos<'a> {
    type Output = Vec<ChapterInfo>;
}

pub struct ListChapterInfosExcluded<'a> {
    pub comic_id: &'a str,
}

impl<'a> Oper for ListChapterInfosExcluded<'a> {
    type Output = Vec<ChapterInfo>;
}

pub struct FindPinnedChapterInfo<'a, 'b> {
    pub comic_id: &'a str,
    pub incls: &'b [ChapterInclOpt],
}

impl<'a, 'b> Oper for FindPinnedChapterInfo<'a, 'b> {
    type Output = Option<ChapterInfo>;
}

pub struct ListPinnedChapterInfos<'a> {
    pub comic_ids: &'a [String],
}

impl<'a> Oper for ListPinnedChapterInfos<'a> {
    type Output = HashMap<String, ChapterInfo>;
}

pub struct UpdateChapter<'a> {
    pub update: &'a ChapterInfoUpdate,
}

impl<'a> Oper for UpdateChapter<'a> {
    type Output = ();
}

pub struct UpdateChapterStage<'a> {
    pub update: &'a ChapterStageUpdate,
}

impl<'a> Oper for UpdateChapterStage<'a> {
    type Output = ();
}

pub struct SetChapterPageCounters<'a> {
    pub id: &'a str,
    pub page_count: i32,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

impl<'a> Oper for SetChapterPageCounters<'a> {
    type Output = ();
}

pub struct AdjustChapterUnitCounters<'a> {
    pub id: &'a str,
    pub delta: UnitCounterDelta,
}

impl<'a> Oper for AdjustChapterUnitCounters<'a> {
    type Output = ();
}

pub struct UnpinOtherChapters<'a> {
    pub comic_id: &'a str,
    pub excluded_id: &'a str,
}

impl<'a> Oper for UnpinOtherChapters<'a> {
    type Output = ();
}

pub struct DeleteChapter<'a> {
    pub id: &'a str,
}

impl<'a> Oper for DeleteChapter<'a> {
    type Output = ();
}
