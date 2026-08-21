use poprako_orchestra::Oper;

use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::spec::comic::ComicListSpec;
use crate::model::write::comic::{
    ComicCoverReservation, ComicEntry, ComicRepl,
};
use crate::value::comic::ComicInclOpt;
use crate::value::image::{ImageExt, ImageHash};

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

/// Lists comics matching the given spec with excluded fields omitted.
#[derive(Oper)]
#[oper(output = Vec<ComicInfo>)]
pub struct ListComicInfosExcluded<'a> {
    /// The filter and pagination specification.
    pub spec: &'a ComicListSpec,
}

/// Updates an existing comic's fields.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpdateComic<'a> {
    /// The update payload.
    pub update: &'a ComicRepl,
}

/// Reserves a cover image slot for a comic.
#[derive(Oper)]
#[oper(output = ComicCoverReservation)]
pub struct ReserveComicCover<'a> {
    //
    /// The comic ID.
    pub id: &'a str,
    /// The hash of the uploaded cover image.
    pub image_hash: &'a ImageHash,
    /// The file extension of the cover image.
    pub image_ext: ImageExt,
}

/// Marks a comic's cover as uploaded or updates its upload state.
#[derive(Oper)]
#[oper(output = ())]
pub struct MarkComicCoverUploaded<'a> {
    //
    /// The comic ID.
    pub id: &'a str,
    /// The cover version to mark.
    pub cover_version: u32,
    /// Optional S3 key for the uploaded cover.
    pub cover_key: Option<&'a str>,
    /// Whether the cover is uploaded.
    pub cover_uploaded: bool,
}

/// Deletes a comic by ID.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteComic<'a> {
    /// The comic ID to delete.
    pub id: &'a str,
}

/// Allocates a new chapter index for a comic.
#[derive(Oper)]
#[oper(output = i32)]
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
