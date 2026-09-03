use poprako_orchestra::Oper;

use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::spec::comic::ComicListSpec;
use crate::model::write::comic::{ComicEntry, ComicRepl};
use crate::value::comic::ComicInclOpt;

/// Creates a new comic with the given entry.
#[derive(Oper)]
#[oper(output = ComicInfo)]
pub struct CreateComic<'a> {
    /// The comic entry to insert.
    pub entry: &'a ComicEntry,
}

/// Retrieves a single comic's info by ID with optional includes.
#[derive(Oper)]
#[oper(output = ComicInfo)]
pub struct GetComicInfo<'a, 'b> {
    //
    /// The comic ID.
    pub id: &'a str,
    /// Which relations to include in the response.
    pub incls: &'b [ComicInclOpt],
}

/// Lists comics matching the given spec.
#[derive(Oper)]
#[oper(output = Vec<ComicInfo>)]
pub struct ListComicInfos<'a> {
    /// The filter and pagination specification.
    pub spec: &'a ComicListSpec,
}

/// Retrieves a single comic's info with excluded relations omitted.
#[derive(Oper)]
#[oper(output = ComicInfo)]
pub struct GetComicInfoExcluded<'a, 'b> {
    //
    /// The comic ID.
    pub id: &'a str,
    /// Which relations to exclude from the response.
    pub incls: &'b [ComicInclOpt],
}

/// Updates an existing comic's fields.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpdateComic<'a> {
    /// The update payload.
    pub update: &'a ComicRepl,
}

/// Allocates a new chapter index for a comic.
#[derive(Oper)]
#[oper(output = usize)]
pub struct AllocComicChapterIndex<'a> {
    /// The comic ID.
    pub id: &'a str,
}

/// Adjusts a comic's chapter count by the given delta.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpdateComicChapterCount<'a> {
    //
    /// The comic ID.
    pub id: &'a str,
    /// The delta to apply (positive or negative).
    pub delta: i32,
}

/// Updates a comic's last-active timestamp to now.
#[derive(Oper)]
#[oper(output = ())]
pub struct TouchComicLastActive<'a> {
    /// The comic ID.
    pub id: &'a str,
}
