use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::write::page_source::PageSourceEntry;
use crate::model::write::team::TeamEntry;
use crate::model::write::user::UserEntry;
use crate::model::write::workset::WorksetEntry;
use crate::shape::write::chapter::ChapterEntryShape;
use crate::shape::write::comic::ComicEntryShape;

#[must_use]
pub fn user_entry(prefix: &str, name: &str) -> UserEntry {
    UserEntry {
        id: [prefix, "user-", name].concat(),
        nickname: [prefix, "user-", name].concat(),
        qid: [prefix, "qid-", name].concat(),
        password_hash: "hash".into(),
    }
}

#[must_use]
pub fn team_entry(prefix: &str) -> TeamEntry {
    TeamEntry {
        id: [prefix, "team"].concat(),
        name: [prefix, "team"].concat(),
        description: "team".into(),
    }
}

#[must_use]
pub fn workset_entry(prefix: &str, team: &TeamEntry) -> WorksetEntry {
    WorksetEntry {
        id: [prefix, "workset"].concat(),
        team_id: team.id.clone(),
        index: 0,
        name: "RDB Workset".into(),
        description: Some("workset".into()),
    }
}

#[must_use]
pub fn comic_entry<'a>(
    prefix: &str,
    workset: &'a WorksetEntry,
    creator_form: &'a UserEntry,
) -> ComicEntryShape<'a> {
    ComicEntryShape {
        id: [prefix, "comic"].concat(),
        workset_id: &workset.id,
        index: 0,
        title: "RDB Comic".into(),
        author: "RDB Author".into(),
        description: Some("comic".into()),
        creator_id: &creator_form.id,
    }
}

#[must_use]
pub fn chapter_entry<'a>(
    prefix: &str,
    comic: &'a ComicInfo,
    creator_form: &'a UserEntry,
) -> ChapterEntryShape<'a> {
    ChapterEntryShape {
        id: [prefix, "chapter"].concat(),
        comic_id: &comic.id,
        is_pinned: true,
        index: 0,
        subtitle: "RDB Chapter".into(),
        creator_id: &creator_form.id,
    }
}

#[must_use]
pub fn page_source_entry(
    prefix: &str,
    chapter: &ChapterInfo,
) -> PageSourceEntry {
    PageSourceEntry {
        id: [prefix, "page"].concat(),
        chapter_id: chapter.id.clone(),
        index: 0,
    }
}
