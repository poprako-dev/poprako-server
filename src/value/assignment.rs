//! Value types for assignment aggregates.

use serde::Deserialize;

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

use crate::value::incl::InclOpt;

/// Incl opts for assignment info queries.
///
/// Each opt embeds additional related data into the returned
/// `AssignmentInfoVal`. Dotted opts implicitly pull in the segments before
/// the dot (e.g. `chapter.comic.workset.team` also embeds `chapter`,
/// `chapter.comic`, and `chapter.comic.workset`).
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub enum AssignmentInclOpt {
    /// Embed the assigned user (`user`).
    #[serde(rename = "user")]
    User,
    /// Embed the chapter (`chapter`).
    #[serde(rename = "chapter")]
    Chapter,
    /// Embed the chapter and its comic (`chapter.comic`; implies `chapter`).
    #[serde(rename = "chapter.comic")]
    ChapterComic,
    /// Embed the chapter, its comic, and the comic's workset
    /// (`chapter.comic.workset`; implies `chapter` and `chapter.comic`).
    #[serde(rename = "chapter.comic.workset")]
    ChapterComicWorkset,
    /// Embed the chapter, its comic, the comic's workset, and the workset's
    /// team (`chapter.comic.workset.team`; implies `chapter`,
    /// `chapter.comic`, and `chapter.comic.workset`).
    #[serde(rename = "chapter.comic.workset.team")]
    ChapterComicWorksetTeam,
    /// Embed the chapter and the chapter's creating user
    /// (`chapter.creator`; implies `chapter`).
    #[serde(rename = "chapter.creator")]
    ChapterCreator,
    /// Embed the chapter, its comic, and the comic's creating user
    /// (`chapter.comic.creator`; implies `chapter` and `chapter.comic`).
    #[serde(rename = "chapter.comic.creator")]
    ChapterComicCreator,
}

impl InclOpt for AssignmentInclOpt {
    fn path(self) -> &'static [Self] {
        match self {
            Self::User => &[Self::User],
            Self::Chapter => &[Self::Chapter],
            Self::ChapterComic => &[Self::Chapter, Self::ChapterComic],
            Self::ChapterComicWorkset => {
                &[Self::Chapter, Self::ChapterComic, Self::ChapterComicWorkset]
            }
            Self::ChapterComicWorksetTeam => &[
                Self::Chapter,
                Self::ChapterComic,
                Self::ChapterComicWorkset,
                Self::ChapterComicWorksetTeam,
            ],
            Self::ChapterCreator => &[Self::Chapter, Self::ChapterCreator],
            Self::ChapterComicCreator => {
                &[Self::Chapter, Self::ChapterComic, Self::ChapterComicCreator]
            }
        }
    }
}
