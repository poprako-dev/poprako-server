use poprako_orchestra::Oper;

use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::unit::UnitCountDelta;
use crate::model::read::spec::chapter::ChapterListSpec;
use crate::model::write::chapter::{
    ChapterEntry, ChapterPatch, ChapterStageRepl,
};
use crate::value::chapter::ChapterInclOpt;
use crate::value::chapter::stage::Stage;

/// Creates a chapter.
#[derive(Oper)]
#[oper(output = ChapterInfo)]
pub struct CreateChapter<'a> {
    /// The chapter entry data.
    pub entry: &'a ChapterEntry,
}

/// Gets a chapter that must exist.
#[derive(Oper)]
#[oper(output = ChapterInfo)]
pub struct GetChapterInfo<'a, 'b> {
    //
    /// Chapter identifier.
    pub id: &'a str,
    /// Chapter inclusion options.
    pub incls: &'b [ChapterInclOpt],
}

/// Gets a chapter that must exist (with exclusive lock).
#[derive(Oper)]
#[oper(output = ChapterInfo)]
pub struct GetChapterInfoExcluded<'a, 'b> {
    //
    /// Chapter identifier.
    pub id: &'a str,
    /// Chapter inclusion options.
    pub incls: &'b [ChapterInclOpt],
}

/// Lists chapter infos selected by a query specification.
#[derive(Oper)]
#[oper(output = Vec<ChapterInfo>)]
pub struct ListChapterInfos<'a> {
    /// Query specification for filtering chapter infos.
    pub spec: &'a ChapterListSpec,
}

/// Lists chapter infos belonging to a comic (with exclusive lock).
#[derive(Oper)]
#[oper(output = Vec<ChapterInfo>)]
pub struct ListChapterInfosExcluded<'a> {
    /// Comic identifier.
    pub comic_id: &'a str,
}

/// Locks all chapter rows belonging to a comic.
#[derive(Oper)]
#[oper(output = ())]
pub struct LockChapters<'a> {
    /// Comic identifier.
    pub comic_id: &'a str,
}

/// Finds the pinned chapter for a comic.
#[derive(Oper)]
#[oper(output = Option<ChapterInfo>)]
pub struct FindPinnedChapterInfo<'a, 'b> {
    //
    /// Comic identifier.
    pub comic_id: &'a str,
    /// Chapter inclusion options.
    pub incls: &'b [ChapterInclOpt],
}

/// Lists pinned chapter infos for the given comics.
#[derive(Oper)]
#[oper(output = Vec<ChapterInfo>)]
pub struct ListPinnedChapterInfos<'a> {
    /// Comic identifiers.
    pub comic_ids: &'a [&'a str],
}

/// Updates a chapter's fields.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpdateChapter<'a> {
    /// The chapter update data.
    pub update: &'a ChapterPatch,
}

/// Updates a chapter's stage.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpdateChapterStage<'a> {
    /// The stage update data.
    pub update: &'a ChapterStageRepl,
}

/// Atomically advances a two-step chapter stage when it is still pending.
#[derive(Oper)]
#[oper(output = bool)]
pub struct StartChapterStage<'a> {
    //
    /// Chapter identifier.
    pub id: &'a str,
    /// Target stage to start.
    pub stage: Stage,
}

/// Resolves raw provision when complete or no longer present.
///
/// Returns `false` only while page uploads are still incomplete.
#[derive(Oper)]
#[oper(output = bool)]
pub struct CompleteChapterRawProvide<'a> {
    /// Chapter identifier.
    pub id: &'a str,
}

/// Sets all page and unit counters for a chapter.
#[derive(Oper)]
#[oper(output = ())]
pub struct SetChapterPageCounters<'a> {
    //
    /// Chapter identifier.
    pub id: &'a str,
    /// Number of pages.
    pub page_count: usize,
    /// Total unit count.
    pub total_unit_count: usize,
    /// Translated unit count.
    pub translated_unit_count: usize,
    /// Proofread unit count.
    pub proofread_unit_count: usize,
}

/// Adjusts the unit counters for a chapter by a delta.
#[derive(Oper)]
#[oper(output = ())]
pub struct AdjustChapterUnitCounters<'a> {
    //
    /// Chapter identifier.
    pub id: &'a str,
    /// Counter delta values.
    pub delta: UnitCountDelta,
}

/// Unpins chapters for a comic, excluding a specific chapter.
#[derive(Oper)]
#[oper(output = ())]
pub struct UnpinOtherChapters<'a> {
    //
    /// Comic identifier.
    pub comic_id: &'a str,
    /// Chapter identifier to exclude from unpinning.
    pub excluded_id: &'a str,
}
