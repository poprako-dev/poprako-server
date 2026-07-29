use std::collections::HashMap;

use poprako_orchestra::Oper;

use crate::model::chapter::{
    ChapterEntry, ChapterInfo, ChapterInfoListSpec, ChapterInfoUpdate,
    ChapterStageUpdate,
};
use crate::model::read::proj::unit::UnitCounterDelta;
use crate::value::chapter::{ChapterInclOpt, Stage};

/// Creates a chapter.
pub struct CreateChapter<'a> {
    /// The chapter entry data.
    pub entry: &'a ChapterEntry,
}

impl Oper for CreateChapter<'_> {
    // Internal output type for this step.
    type Output = ChapterInfo;
}

/// Gets a chapter that must exist.
pub struct GetChapterInfo<'a, 'b> {
    //
    /// Chapter identifier.
    pub id: &'a str,
    /// Chapter inclusion options.
    pub incls: &'b [ChapterInclOpt],
}

impl Oper for GetChapterInfo<'_, '_> {
    // Internal output type for this step.
    type Output = ChapterInfo;
}

/// Gets a chapter that must exist (with exclusive lock).
pub struct GetChapterInfoExcluded<'a, 'b> {
    //
    /// Chapter identifier.
    pub id: &'a str,
    /// Chapter inclusion options.
    pub incls: &'b [ChapterInclOpt],
}

impl Oper for GetChapterInfoExcluded<'_, '_> {
    // Internal output type for this step.
    type Output = ChapterInfo;
}

/// Lists chapter infos selected by a query specification.
pub struct ListChapterInfos<'a> {
    /// Query specification for filtering chapter infos.
    pub spec: &'a ChapterInfoListSpec,
}

impl Oper for ListChapterInfos<'_> {
    // Internal output type for this step.
    type Output = Vec<ChapterInfo>;
}

/// Lists chapter infos belonging to a comic (with exclusive lock).
pub struct ListChapterInfosExcluded<'a> {
    /// Comic identifier.
    pub comic_id: &'a str,
}

impl Oper for ListChapterInfosExcluded<'_> {
    // Internal output type for this step.
    type Output = Vec<ChapterInfo>;
}

/// Locks all chapter rows belonging to a comic.
pub struct LockChapters<'a> {
    /// Comic identifier.
    pub comic_id: &'a str,
}

impl Oper for LockChapters<'_> {
    // Internal output type for this step.
    type Output = ();
}

/// Finds the pinned chapter for a comic.
pub struct FindPinnedChapterInfo<'a, 'b> {
    //
    /// Comic identifier.
    pub comic_id: &'a str,
    /// Chapter inclusion options.
    pub incls: &'b [ChapterInclOpt],
}

impl Oper for FindPinnedChapterInfo<'_, '_> {
    // Internal output type for this step.
    type Output = Option<ChapterInfo>;
}

/// Lists pinned chapter infos for the given comics.
pub struct ListPinnedChapterInfos<'a> {
    /// Comic identifiers.
    pub comic_ids: &'a [String],
}

impl Oper for ListPinnedChapterInfos<'_> {
    // Internal output type for this step.
    type Output = HashMap<String, ChapterInfo>;
}

/// Updates a chapter's fields.
pub struct UpdateChapter<'a> {
    /// The chapter update data.
    pub update: &'a ChapterInfoUpdate,
}

impl Oper for UpdateChapter<'_> {
    // Internal output type for this step.
    type Output = ();
}

/// Updates a chapter's stage.
pub struct UpdateChapterStage<'a> {
    /// The stage update data.
    pub update: &'a ChapterStageUpdate,
}

impl Oper for UpdateChapterStage<'_> {
    // Internal output type for this step.
    type Output = ();
}

/// Atomically starts a two-step chapter stage when it is still pending.
pub struct StartChapterStage<'a> {
    //
    /// Chapter identifier.
    pub id: &'a str,
    /// Target stage to start.
    pub stage: Stage,
}

impl Oper for StartChapterStage<'_> {
    // Internal output type for this step.
    type Output = bool;
}

/// Resolves raw provision when complete or no longer present.
///
/// Returns `false` only while page uploads are still incomplete.
pub struct CompleteChapterRawProvide<'a> {
    /// Chapter identifier.
    pub id: &'a str,
}

impl Oper for CompleteChapterRawProvide<'_> {
    // Internal output type for this step.
    type Output = bool;
}

/// Clears raw-provision completion without changing any other stage.
pub struct ResetChapterRawProvide<'a> {
    /// Chapter identifier.
    pub id: &'a str,
}

impl Oper for ResetChapterRawProvide<'_> {
    // Internal output type for this step.
    type Output = ();
}

/// Sets all page and unit counters for a chapter.
pub struct SetChapterPageCounters<'a> {
    //
    /// Chapter identifier.
    pub id: &'a str,
    /// Number of pages.
    pub page_count: i32,
    /// Total unit count.
    pub total_unit_count: i32,
    /// Translated unit count.
    pub translated_unit_count: i32,
    /// Proofread unit count.
    pub proofread_unit_count: i32,
}

impl Oper for SetChapterPageCounters<'_> {
    // Internal output type for this step.
    type Output = ();
}

/// Adjusts the unit counters for a chapter by a delta.
pub struct AdjustChapterUnitCounters<'a> {
    //
    /// Chapter identifier.
    pub id: &'a str,
    /// Counter delta values.
    pub delta: UnitCounterDelta,
}

impl Oper for AdjustChapterUnitCounters<'_> {
    // Internal output type for this step.
    type Output = ();
}

/// Unpins chapters for a comic, excluding a specific chapter.
pub struct UnpinOtherChapters<'a> {
    //
    /// Comic identifier.
    pub comic_id: &'a str,
    /// Chapter identifier to exclude from unpinning.
    pub excluded_id: &'a str,
}

impl Oper for UnpinOtherChapters<'_> {
    // Internal output type for this step.
    type Output = ();
}

/// Deletes a chapter.
pub struct DeleteChapter<'a> {
    /// Chapter identifier.
    pub id: &'a str,
}

impl Oper for DeleteChapter<'_> {
    // Internal output type for this step.
    type Output = ();
}
