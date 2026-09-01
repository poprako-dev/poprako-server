//! Page use-case tests.

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;
use time::OffsetDateTime;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::{Mock, MockObjRecord};
use crate::value::chapter::mask::StageMask;
use crate::value::image::{ImageExt, ImageHash};
use crate::value::role::RoleMask;

// Page upload availability tests.
mod mark_uploaded;
// Batch page-manifest reservation tests.
mod alloc;
// Single page-image allocation tests.
mod image_alloc;
// Page-manifest lifecycle edge-case tests.
mod manifest_lifecycle;
// Page deletion tests.
mod delete;
// Proofread-diff Page ID query tests.
mod editted_diff;
// Validation guard tests for page operations.
mod validation;
// Batch page-image presentation tests.
mod view;

fn page_token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn page_workset() -> WorksetInfo {
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: "workset-1".into(),
        team_id: "team-1".into(),
        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 1,
        created_at: time,
        updated_at: time,
    }
}

fn page_comic() -> ComicInfo {
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: "comic-1".into(),
        workset_id: "workset-1".into(),
        index: 0,
        title: "comic".into(),
        author: "author".into(),
        description: None,
        chapter_count: 1,
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

fn page_chapter(page_count: usize) -> ChapterInfo {
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: "chapter-1".into(),
        comic_id: "comic-1".into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "chapter".into(),
        page_count,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: StageMask::try_from(0u32).unwrap(),
        creator_id: "user-1".into(),
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

fn page_member(user_id: &str, roles: RoleMask) -> MemberInfo {
    MemberInfo {
        id: format!("member-{user_id}"),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: "team-1".into(),
        user: None,
        team: None,
        roles,
    }
}

fn page_assignment(user_id: &str, roles: RoleMask) -> AssignmentInfo {
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: format!("assignment-{user_id}"),
        chapter_id: "chapter-1".into(),
        user_id: user_id.into(),
        user: None,
        chapter: None,
        roles,
        created_at: time,
        updated_at: time,
    }
}

fn page_model(id: &str, index: usize) -> PageInfo {
    let time = OffsetDateTime::now_utc();

    PageInfo {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        index,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    }
}

fn seed_page_scope(mock: &Mock, page_count: usize) {
    mock.seed_workset(page_workset());

    mock.seed_comic(page_comic());

    mock.seed_chapter(page_chapter(page_count));
}

fn seed_page_obj(
    mock: &Mock,
    id: &str,
    version: u32,
    is_avail: bool,
    hash: u8,
    ext: ImageExt,
) -> ObjKey {
    let key = ObjKey {
        id: id.into(),
        ver: version,
        image: format!(
            "page/chapter_chapter-1/{id}-{version}.{}",
            ext.suffix()
        ),
    };

    let meta = ObjMeta {
        key: key.clone(),
        is_avail,
        hash: vec![hash; 32],
        ext: ext.suffix().into(),
    };

    mock.state
        .lock()
        .unwrap()
        .objs
        .entry("page_image")
        .or_default()
        .insert(
            id.into(),
            MockObjRecord {
                version,
                meta: Some(meta),
            },
        );

    key
}
