use super::*;

use time::Duration;

use crate::data::instr::comic::CreateComicInstr;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::shared::user::UserToken;
use crate::value::role::{RoleField, RoleMask};

pub fn comic(id: &str, workset_id: &str, index: usize) -> ComicInfo {
    //
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index,
        title: ["comic-", &index.to_string()].concat(),
        author: "author".into(),
        description: None,
        chapter_count: 0,
        creator_id: "user--00000000001".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        archived_at: None,
        created_at: time,
        updated_at: time,
    }
}

pub fn comics_with_opposed_activity(
    baseline: OffsetDateTime,
) -> [ComicInfo; 2] {
    [
        ComicInfo {
            last_active_at: baseline - Duration::hours(1),
            updated_at: baseline + Duration::hours(1),
            ..comic("comic--00000000002", "workset--00000000001", 2)
        },
        ComicInfo {
            last_active_at: baseline,
            updated_at: baseline - Duration::hours(1),
            ..comic("comic--00000000001", "workset--00000000001", 1)
        },
    ]
}

pub fn chapter(id: &str, comic_id: &str, stage_mask: StageMask) -> ChapterInfo {
    //
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        confirmed_artwork_ver: None,
        id: id.into(),
        comic_id: comic_id.into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "chapter".into(),
        page_source_count: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: stage_mask,
        creator_id: "user--00000000001".into(),
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

pub fn assignment(id: &str, chapter_id: &str, user_id: &str) -> AssignmentInfo {
    //
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: id.into(),
        chapter_id: chapter_id.into(),
        user_id: user_id.into(),
        user: None,
        chapter: None,
        roles: RoleMask::from(RoleField::TRANSLATOR),
        created_at: time,
        updated_at: time,
    }
}

pub fn create_instr(workset_id: &str) -> CreateComicInstr {
    CreateComicInstr {
        workset_id: workset_id.into(),
        title: "new".into(),
        author: "author".into(),
        description: Some("desc".into()),
        first_chapter_subtitle: None,
        preset_assignment_roles: None,
    }
}

pub fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

pub fn admin_member(user_id: &str, team_id: &str) -> MemberInfo {
    MemberInfo {
        id: ["member-", user_id, "-", team_id].concat(),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: RoleMask::from(RoleField::ADMIN),
    }
}
