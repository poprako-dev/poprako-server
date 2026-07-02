//! Value types for assignment aggregates.

use serde::Deserialize;

use crate::value::incl::InclOpt;

/// Incl opts for assignment info queries.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentInclOpt {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "chapter")]
    Chapter,
    #[serde(rename = "chapter.comic")]
    ChapterComic,
    #[serde(rename = "chapter.comic.workset")]
    ChapterComicWorkset,
    #[serde(rename = "chapter.comic.workset.team")]
    ChapterComicWorksetTeam,
    #[serde(rename = "chapter.creator")]
    ChapterCreator,
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
