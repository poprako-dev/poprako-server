use super::*;

use time::OffsetDateTime;

use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::member::MemberInfo;
use crate::model::page::PageInfo;
use crate::model::user::UserToken;
use crate::model::workset::WorksetInfo;
use crate::value::chapter::StageMask;

pub fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

pub fn workset(id: &str, team_id: &str) -> WorksetInfo {
    //
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 1,
        created_at: time,
        updated_at: time,
    }
}

pub fn comic(id: &str, workset_id: &str) -> ComicInfo {
    //
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 1,
        title: "comic".into(),
        author: "author".into(),
        description: None,
        cover_key: None,
        cover_uploaded: false,
        cover_version: 0,
        chapter_count: 2,
        creator_id: "user-1".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

pub fn chapter(
    id: &str,
    comic_id: &str,
    index: i32,
    is_pinned: bool,
) -> ChapterInfo {
    //
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: id.into(),
        comic_id: comic_id.into(),
        comic: None,
        is_pinned,
        index,
        subtitle: format!("chapter {}", index),
        page_count: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: StageMask::try_from(0u32).ok().unwrap(),
        creator_id: "user-1".into(),
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

pub fn member(user_id: &str, team_id: &str, role_mask: RoleMask) -> MemberInfo {
    MemberInfo {
        id: format!("member-{}-{}", user_id, team_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: role_mask,
    }
}

pub fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> AssignmentInfo {
    //
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: format!("assignment-{}-{}", chapter_id, user_id),
        chapter_id: chapter_id.into(),
        user_id: user_id.into(),
        user: None,
        chapter: None,
        roles: role_mask,
        created_at: time,
        updated_at: time,
    }
}

pub fn page(id: &str, chapter_id: &str, image_key: Option<&str>) -> PageInfo {
    //
    let time = OffsetDateTime::now_utc();

    PageInfo {
        id: id.into(),
        chapter_id: chapter_id.into(),
        index: 0,
        image_key: image_key.map(Into::into),
        image_uploaded: image_key.is_some(),
        image_version: 1,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    }
}

pub fn seed_scope(mock: &Mock, user_id: &str, role_mask: RoleMask) {
    //
    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1"));

    mock.seed_member(member(user_id, "team-1", role_mask));
}
