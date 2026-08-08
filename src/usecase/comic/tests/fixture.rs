use super::*;
use crate::data::instr::comic::CreateComicInstr;
use crate::value::image::{ImageExt, ImageHash};

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::shared::user::UserToken;
use crate::value::role::{RoleField, RoleMask};

pub fn comic(id: &str, workset_id: &str, index: i32) -> ComicInfo {
    //
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index,
        title: format!("comic-{}", index),
        author: "author".into(),
        description: None,
        cover_key: None,
        is_cover_uploaded: None,
        cover_version: None,
        cover_hash: None,
        cover_ext: None,
        chapter_count: 0,
        creator_id: "user-1".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        archived_at: None,
        created_at: time,
        updated_at: time,
    }
}

pub fn comic_with_uploaded_cover(
    id: &str,
    workset_id: &str,
    cover_key: &str,
) -> ComicInfo {
    ComicInfo {
        cover_key: Some(cover_key.into()),
        is_cover_uploaded: Some(true),
        cover_version: Some(1),
        cover_hash: Some(ImageHash::default()),
        cover_ext: Some(ImageExt::Png),
        ..comic(id, workset_id, 0)
    }
}

pub fn chapter(id: &str, comic_id: &str, stage_mask: StageMask) -> ChapterInfo {
    //
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: id.into(),
        comic_id: comic_id.into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "chapter".into(),
        page_count: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: stage_mask,
        creator_id: "user-1".into(),
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

pub fn page(
    id: &str,
    chapter_id: &str,
    index: i32,
    image_key: Option<&str>,
    image_uploaded: bool,
) -> PageInfo {
    //
    let time = OffsetDateTime::now_utc();

    PageInfo {
        id: id.into(),
        chapter_id: chapter_id.into(),
        index,
        image_key: image_key.map(Into::into),
        is_image_uploaded: Some(image_uploaded),
        image_version: Some(1),
        image_hash: Some(ImageHash::new([0u8; 32])),
        image_ext: Some(ImageExt::Png),
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
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
        id: format!("member-{}-{}", user_id, team_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: RoleMask::from(RoleField::ADMIN),
    }
}
