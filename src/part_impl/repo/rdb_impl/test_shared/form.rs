use crate::model::chapter::ChapterEntry;
use crate::model::comic::ComicEntry;
use crate::model::page::PageEntry;
use crate::value::image::ImageExt;
use crate::model::team::TeamEntry;
use crate::model::user::UserEntry;
use crate::model::workset::WorksetEntry;

pub fn user_entry(prefix: &str, name: &str) -> UserEntry {
    UserEntry {
        id: format!("{}user-{}", prefix, name),
        nickname: format!("{}user-{}", prefix, name),
        qid: format!("{}qid-{}", prefix, name),
        password_hash: "hash".into(),
    }
}

pub fn team_entry(prefix: &str) -> TeamEntry {
    TeamEntry {
        id: format!("{}team", prefix),
        name: format!("{}team", prefix),
        description: "team".into(),
    }
}

pub fn workset_entry(prefix: &str, team_entry: &TeamEntry) -> WorksetEntry {
    WorksetEntry {
        id: format!("{}workset", prefix),
        team_id: team_entry.id.clone(),
        index: 0,
        name: "RDB Workset".into(),
        description: Some("workset".into()),
    }
}

pub fn comic_entry(
    prefix: &str,
    workset_entry: &WorksetEntry,
    creator_form: &UserEntry,
) -> ComicEntry {
    ComicEntry {
        id: format!("{}comic", prefix),
        workset_id: workset_entry.id.clone(),
        index: 0,
        title: "RDB Comic".into(),
        author: "RDB Author".into(),
        description: Some("comic".into()),
        creator_id: creator_form.id.clone(),
    }
}

pub fn chapter_entry(
    prefix: &str,
    comic_entry: &ComicEntry,
    creator_form: &UserEntry,
) -> ChapterEntry {
    ChapterEntry {
        id: format!("{}chapter", prefix),
        comic_id: comic_entry.id.clone(),
        is_pinned: true,
        index: 0,
        subtitle: "RDB Chapter".into(),
        creator_id: creator_form.id.clone(),
    }
}

pub fn page_entry(prefix: &str, chapter_entry: &ChapterEntry) -> PageEntry {
    PageEntry {
        id: format!("{}page", prefix),
        chapter_id: chapter_entry.id.clone(),
        index: 0,
        image_key: None,
        image_version: 0,
        image_hash: Default::default(),
        image_byte_len: 1,
        image_ext: ImageExt::Jpg,
    }
}
