use std::collections::HashMap;

use poprako_orchestra::Oper;

use crate::model::chapter::{
    ChapterEntry, ChapterInfo, ChapterInfoListSpec, ChapterInfoUpdate,
    ChapterStageUpdate,
};
use crate::model::unit::UnitCounterDelta;
use crate::value::chapter::{ChapterInclOpt, Stage};

pub struct CreateChapter<'a> {
    pub entry: &'a ChapterEntry,
}

impl Oper for CreateChapter<'_> {
    type Output = ChapterInfo;
}

pub struct GetChapterInfo<'a, 'b> {
    //
    pub id: &'a str,
    pub incls: &'b [ChapterInclOpt],
}

impl Oper for GetChapterInfo<'_, '_> {
    type Output = ChapterInfo;
}

pub struct GetChapterInfoExcluded<'a, 'b> {
    //
    pub id: &'a str,
    pub incls: &'b [ChapterInclOpt],
}

impl Oper for GetChapterInfoExcluded<'_, '_> {
    type Output = ChapterInfo;
}

pub struct ListChapterInfos<'a> {
    pub spec: &'a ChapterInfoListSpec,
}

impl Oper for ListChapterInfos<'_> {
    type Output = Vec<ChapterInfo>;
}

pub struct ListChapterInfosExcluded<'a> {
    pub comic_id: &'a str,
}

impl Oper for ListChapterInfosExcluded<'_> {
    type Output = Vec<ChapterInfo>;
}

/// Locks all chapter rows belonging to a comic.
pub struct LockChapters<'a> {
    pub comic_id: &'a str,
}

impl Oper for LockChapters<'_> {
    type Output = ();
}

pub struct FindPinnedChapterInfo<'a, 'b> {
    //
    pub comic_id: &'a str,
    pub incls: &'b [ChapterInclOpt],
}

impl Oper for FindPinnedChapterInfo<'_, '_> {
    type Output = Option<ChapterInfo>;
}

pub struct ListPinnedChapterInfos<'a> {
    pub comic_ids: &'a [String],
}

impl Oper for ListPinnedChapterInfos<'_> {
    type Output = HashMap<String, ChapterInfo>;
}

pub struct UpdateChapter<'a> {
    pub update: &'a ChapterInfoUpdate,
}

impl Oper for UpdateChapter<'_> {
    type Output = ();
}

pub struct UpdateChapterStage<'a> {
    pub update: &'a ChapterStageUpdate,
}

impl Oper for UpdateChapterStage<'_> {
    type Output = ();
}

/// Atomically starts a two-step chapter stage when it is still pending.
pub struct StartChapterStage<'a> {
    //
    pub id: &'a str,
    pub stage: Stage,
}

impl Oper for StartChapterStage<'_> {
    type Output = bool;
}

/// Resolves raw provision when complete or no longer present.
///
/// Returns `false` only while page uploads are still incomplete.
pub struct CompleteChapterRawProvide<'a> {
    pub id: &'a str,
}

impl Oper for CompleteChapterRawProvide<'_> {
    type Output = bool;
}

/// Clears raw-provision completion without changing any other stage.
pub struct ResetChapterRawProvide<'a> {
    pub id: &'a str,
}

impl Oper for ResetChapterRawProvide<'_> {
    type Output = ();
}

pub struct SetChapterPageCounters<'a> {
    //
    pub id: &'a str,
    pub page_count: i32,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
}

impl Oper for SetChapterPageCounters<'_> {
    type Output = ();
}

pub struct AdjustChapterUnitCounters<'a> {
    //
    pub id: &'a str,
    pub delta: UnitCounterDelta,
}

impl Oper for AdjustChapterUnitCounters<'_> {
    type Output = ();
}

pub struct UnpinOtherChapters<'a> {
    //
    pub comic_id: &'a str,
    pub excluded_id: &'a str,
}

impl Oper for UnpinOtherChapters<'_> {
    type Output = ();
}

pub struct DeleteChapter<'a> {
    pub id: &'a str,
}

impl Oper for DeleteChapter<'_> {
    type Output = ();
}
