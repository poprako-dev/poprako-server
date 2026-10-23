use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::write::page::PageEntry;
use crate::model::write::team::TeamEntry;
use crate::model::write::user::UserEntry;
use crate::model::write::workset::WorksetEntry;
use crate::shape::write::chapter::ChapterEntryShape;
use crate::shape::write::comic::ComicEntryShape;

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
#[must_use]
pub fn user_entry(prefix: &str, name: &str) -> UserEntry {
    UserEntry {
        id: format!("{}user-{}", prefix, name),
        nickname: format!("{}user-{}", prefix, name),
        qid: format!("{}qid-{}", prefix, name),
        password_hash: "hash".into(),
    }
}

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
#[must_use]
pub fn team_entry(prefix: &str) -> TeamEntry {
    TeamEntry {
        id: format!("{}team", prefix),
        name: format!("{}team", prefix),
        description: "team".into(),
    }
}

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
#[must_use]
pub fn workset_entry(prefix: &str, team: &TeamEntry) -> WorksetEntry {
    WorksetEntry {
        id: format!("{}workset", prefix),
        team_id: team.id.clone(),
        index: 0,
        name: "RDB Workset".into(),
        description: Some("workset".into()),
    }
}

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
#[must_use]
pub fn comic_entry<'a>(
    prefix: &str,
    workset: &'a WorksetEntry,
    creator_form: &'a UserEntry,
) -> ComicEntryShape<'a> {
    ComicEntryShape {
        id: format!("{}comic", prefix),
        workset_id: &workset.id,
        index: 0,
        title: "RDB Comic".into(),
        author: "RDB Author".into(),
        description: Some("comic".into()),
        creator_id: &creator_form.id,
    }
}

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
#[must_use]
pub fn chapter_entry<'a>(
    prefix: &str,
    comic: &'a ComicInfo,
    creator_form: &'a UserEntry,
) -> ChapterEntryShape<'a> {
    ChapterEntryShape {
        id: format!("{}chapter", prefix),
        comic_id: &comic.id,
        is_pinned: true,
        index: 0,
        subtitle: "RDB Chapter".into(),
        creator_id: &creator_form.id,
    }
}

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
#[must_use]
pub fn page_entry(prefix: &str, chapter: &ChapterInfo) -> PageEntry {
    PageEntry {
        id: format!("{}page", prefix),
        chapter_id: chapter.id.clone(),
        index: 0,
    }
}
