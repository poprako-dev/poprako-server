use poprako_orchestra::Oper;

use crate::model::comic::{
    ComicCoverReservation, ComicEntry, ComicInfo, ComicInfoListSpec,
    ComicInfoUpdate,
};
use crate::value::comic::ComicInclOpt;
use crate::value::image::{ImageExt, ImageHash};

/// Creates a new comic with the given entry.
pub struct CreateComic<'a> {
    /// The comic entry to insert.
    pub entry: &'a ComicEntry,
}

impl Oper for CreateComic<'_> {
    // Created comic info.
    type Output = ComicInfo;
}

/// Retrieves a single comic's info by ID with optional includes.
pub struct GetComicInfo<'a, 'b> {
    //
    /// The comic ID.
    pub id: &'a str,
    /// Which relations to include in the response.
    pub incls: &'b [ComicInclOpt],
}

impl Oper for GetComicInfo<'_, '_> {
    // Retrieved comic info.
    type Output = ComicInfo;
}

/// Lists comics matching the given spec.
pub struct ListComicInfos<'a> {
    /// The filter and pagination specification.
    pub spec: &'a ComicInfoListSpec,
}

impl Oper for ListComicInfos<'_> {
    // List of matching comic infos.
    type Output = Vec<ComicInfo>;
}

/// Retrieves a single comic's info with excluded relations omitted.
pub struct GetComicInfoExcluded<'a, 'b> {
    //
    /// The comic ID.
    pub id: &'a str,
    /// Which relations to exclude from the response.
    pub incls: &'b [ComicInclOpt],
}

impl Oper for GetComicInfoExcluded<'_, '_> {
    // Retrieved comic info with excluded fields omitted.
    type Output = ComicInfo;
}

/// Lists comics matching the given spec with excluded fields omitted.
pub struct ListComicInfosExcluded<'a> {
    /// The filter and pagination specification.
    pub spec: &'a ComicInfoListSpec,
}

impl Oper for ListComicInfosExcluded<'_> {
    // List of matching comic infos with excluded fields omitted.
    type Output = Vec<ComicInfo>;
}

/// Updates an existing comic's fields.
pub struct UpdateComic<'a> {
    /// The update payload.
    pub update: &'a ComicInfoUpdate,
}

impl Oper for UpdateComic<'_> {
    // Unit on success.
    type Output = ();
}

/// Reserves a cover image slot for a comic.
pub struct ReserveComicCover<'a> {
    //
    /// The comic ID.
    pub id: &'a str,
    /// The hash of the uploaded cover image.
    pub image_hash: &'a ImageHash,
    /// The file extension of the cover image.
    pub image_ext: ImageExt,
}

impl Oper for ReserveComicCover<'_> {
    // The cover reservation details.
    type Output = ComicCoverReservation;
}

/// Marks a comic's cover as uploaded or updates its upload state.
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

impl Oper for MarkComicCoverUploaded<'_> {
    // Unit on success.
    type Output = ();
}

/// Deletes a comic by ID.
pub struct DeleteComic<'a> {
    /// The comic ID to delete.
    pub id: &'a str,
}

impl Oper for DeleteComic<'_> {
    // Unit on success.
    type Output = ();
}

/// Allocates a new chapter index for a comic.
pub struct AllocComicChapterIndex<'a> {
    /// The comic ID.
    pub id: &'a str,
}

impl Oper for AllocComicChapterIndex<'_> {
    // The allocated chapter index.
    type Output = i32;
}

/// Adjusts a comic's chapter count by the given delta.
pub struct UpdateComicChapterCount<'a> {
    //
    /// The comic ID.
    pub id: &'a str,
    /// The delta to apply (positive or negative).
    pub delta: i32,
}

impl Oper for UpdateComicChapterCount<'_> {
    // Unit on success.
    type Output = ();
}

/// Updates a comic's last-active timestamp to now.
pub struct TouchComicLastActive<'a> {
    /// The comic ID.
    pub id: &'a str,
}

impl Oper for TouchComicLastActive<'_> {
    // Unit on success.
    type Output = ();
}
