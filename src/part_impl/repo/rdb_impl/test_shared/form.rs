use crate::model::write::chapter::ChapterEntry;
use crate::model::write::comic::ComicEntry;
use crate::model::write::page::PageEntry;
use crate::model::write::team::TeamEntry;
use crate::model::write::user::UserEntry;
use crate::model::write::workset::WorksetEntry;

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
#[must_use]
pub fn user_entry(prefix: &str, name: &str) -> UserEntry<'static> {
    UserEntry {
        id: format!("{}user-{}", prefix, name),
        nickname: format!("{}user-{}", prefix, name).into(),
        qid: format!("{}qid-{}", prefix, name).into(),
        password_hash: "hash".into(),
    }
}

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
#[must_use]
pub fn team_entry(prefix: &str) -> TeamEntry<'static> {
    TeamEntry {
        id: format!("{}team", prefix),
        name: format!("{}team", prefix).into(),
        description: "team".into(),
    }
}

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
#[must_use]
pub fn workset_entry(
    prefix: &str,
    team_entry: &TeamEntry<'_>,
) -> WorksetEntry<'static> {
    WorksetEntry {
        id: format!("{}workset", prefix),
        team_id: team_entry.id.clone(),
        index: 0,
        name: "RDB Workset",
        description: Some("workset"),
    }
}

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
#[must_use]
pub fn comic_entry(
    prefix: &str,
    workset_entry: &WorksetEntry<'_>,
    creator_form: &UserEntry<'_>,
) -> ComicEntry<'static> {
    ComicEntry {
        id: format!("{}comic", prefix),
        workset_id: workset_entry.id.clone().into(),
        index: 0,
        title: "RDB Comic",
        author: "RDB Author",
        description: Some("comic"),
        creator_id: creator_form.id.clone().into(),
    }
}

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
#[must_use]
pub fn chapter_entry(
    prefix: &str,
    comic_entry: &ComicEntry<'_>,
    creator_form: &UserEntry<'_>,
) -> ChapterEntry<'static> {
    ChapterEntry {
        id: format!("{}chapter", prefix),
        comic_id: comic_entry.id.clone().into(),
        is_pinned: true,
        index: 0,
        subtitle: "RDB Chapter".into(),
        creator_id: creator_form.id.clone().into(),
    }
}

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
#[must_use]
pub fn page_entry(prefix: &str, chapter_entry: &ChapterEntry<'_>) -> PageEntry {
    PageEntry {
        id: format!("{}page", prefix),
        chapter_id: chapter_entry.id.clone(),
        index: 0,
    }
}
