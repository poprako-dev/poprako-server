// list_infos(list_infos)(positive): team member lists page units in index order with page counters.
// list_infos(list_infos)(negative): non-member without assignment cannot list page units.
// save_infos(save_infos)(positive): translator saves ordered operations, id mappings, counters, and comic activity.
// save_infos(save_infos)(negative): user without edit role rolls back units and counters.

use super::*;

use time::OffsetDateTime;

use crate::data::unit::{ListPageUnitInfosData, SavePageUnitsData, UnitOperationData};
use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::member::MemberInfo;
use crate::model::page::PageInfo;
use crate::model::role::{RoleField, RoleMask};
use crate::model::unit::{UnitInfo, UnitLocalSnapshot, UnitServerSnapshot};
use crate::model::user::UserToken;
use crate::model::workset::WorksetInfo;
use crate::part_impl::repo_mock::Mock;
use crate::result::{ExpectedVariant, RootError};
use crate::value::chapter::WorkflowStageMask;

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn workset(id: &str, team_id: &str) -> WorksetInfo {
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 1,
        comic_next_index: 1,
        created_at: time,
        updated_at: time,
    }
}

fn comic(id: &str, workset_id: &str) -> ComicInfo {
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 0,
        title: "comic".into(),
        author: "author".into(),
        description: None,
        is_completed: false,
        cover_key: None,
        cover_uploaded: false,
        cover_version: 0,
        chapter_count: 1,
        chapter_next_index: 1,
        creator_id: "user-1".into(),
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn chapter(id: &str, comic_id: &str, total: i32, translated: i32, proofread: i32) -> ChapterInfo {
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: id.into(),
        comic_id: comic_id.into(),
        is_pinned: true,
        index: 0,
        subtitle: "chapter".into(),
        page_count: 1,
        total_unit_count: total,
        translated_unit_count: translated,
        proofread_unit_count: proofread,
        stages: WorkflowStageMask::try_from(0u32).ok().unwrap(),
        creator_id: "user-1".into(),
        created_at: time,
        updated_at: time,
    }
}

fn member(user_id: &str) -> MemberInfo {
    MemberInfo {
        id: format!("member-{}", user_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        team_id: "team-1".into(),
        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn assignment(chapter_id: &str, user_id: &str, role_mask: RoleMask) -> AssignmentInfo {
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: format!("assignment-{}-{}", chapter_id, user_id),
        chapter_id: chapter_id.into(),
        user_id: user_id.into(),
        roles: role_mask,
        created_at: time,
        updated_at: time,
    }
}

fn page(id: &str, total: i32, translated: i32, proofread: i32) -> PageInfo {
    let time = OffsetDateTime::now_utc();

    PageInfo {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        index: 0,
        image_key: None,
        image_uploaded: false,
        image_version: 0,
        total_unit_count: total,
        translated_unit_count: translated,
        proofread_unit_count: proofread,
        created_at: time,
        updated_at: time,
    }
}

fn unit(
    id: &str,
    index: i32,
    text: &str,
    proofread_text: Option<&str>,
    proofread: bool,
) -> UnitInfo {
    let time = OffsetDateTime::now_utc();

    UnitInfo {
        id: id.into(),
        page_id: "page-1".into(),
        index,
        is_bubble: true,
        is_proofread: proofread,
        x_coord: 1.0,
        y_coord: 2.0,
        translated_text: Some(text.into()),
        translator_comment: None,
        last_translator_id: None,
        proofread_text: proofread_text.map(Into::into),
        proofreader_comment: None,
        last_proofreader_id: None,
        created_at: time,
        updated_at: time,
    }
}

fn server_snapshot(id: &str, text: &str) -> UnitServerSnapshot {
    UnitServerSnapshot {
        id: id.into(),
        is_bubble: true,
        is_proofread: false,
        x_coord: 1.0,
        y_coord: 2.0,
        translated_text: Some(text.into()),
        translator_comment: None,
        last_translator_id: None,
        proofread_text: None,
        proofreader_comment: None,
        last_proofreader_id: None,
    }
}

fn local_snapshot(id: &str, text: &str, proofread: bool) -> UnitLocalSnapshot {
    UnitLocalSnapshot {
        local_id: id.into(),
        is_bubble: true,
        is_proofread: proofread,
        x_coord: 3.0,
        y_coord: 4.0,
        translated_text: Some(text.into()),
        translator_comment: None,
        last_translator_id: None,
        proofread_text: None,
        proofreader_comment: None,
        last_proofreader_id: None,
    }
}

fn seed_scope(mock: &Mock, total: i32, translated: i32, proofread: i32) {
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_comic(comic("comic-1", "workset-1"));
    mock.seed_chapter(chapter(
        "chapter-1",
        "comic-1",
        total,
        translated,
        proofread,
    ));
    mock.seed_page(page("page-1", total, translated, proofread));
}

#[tokio::test]
async fn list_infos_returns_units_for_team_member() {
    let mock = Mock::new();
    seed_scope(&mock, 2, 1, 1);
    mock.seed_member(member("user-1"));
    mock.seed_unit(unit("unit-b", 1, "beta", None, false));
    mock.seed_unit(unit("unit-a", 0, "", Some("proof"), true));

    let listed = list_infos(
        &mock,
        token("user-1"),
        ListPageUnitInfosData {
            page_id: "page-1".into(),
        },
    )
    .await;

    let listed = match listed {
        Ok(listed) => listed,
        Err(_) => panic!("expected list success"),
    };

    assert_eq!(listed.units.len(), 2);
    assert_eq!(listed.units[0].id, "unit-a");
    assert_eq!(listed.units[1].id, "unit-b");
    assert_eq!(listed.total_unit_count, 2);
    assert_eq!(listed.translated_unit_count, 1);
    assert_eq!(listed.proofread_unit_count, 1);
}

#[tokio::test]
async fn list_infos_rejects_unrelated_user() {
    let mock = Mock::new();
    seed_scope(&mock, 0, 0, 0);

    let err = list_infos(
        &mock,
        token("user-2"),
        ListPageUnitInfosData {
            page_id: "page-1".into(),
        },
    )
    .await
    .err()
    .unwrap();

    match err {
        RootError::Expected { variant, .. } => {
            assert!(matches!(variant, ExpectedVariant::Perm));
        }
        RootError::Unrecoverable { .. } => {
            panic!("expected permission error");
        }
    }
}

#[tokio::test]
async fn save_infos_applies_operations_and_updates_counters() {
    let mock = Mock::new();
    seed_scope(&mock, 2, 2, 1);
    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));
    mock.seed_unit(unit("unit-a", 0, "alpha", None, false));
    mock.seed_unit(unit("unit-b", 1, "beta", Some("proof"), true));

    let saved = save_infos(
        &mock,
        &mock,
        token("user-1"),
        SavePageUnitsData {
            page_id: "page-1".into(),
            opers: vec![
                UnitOperationData::Delete {
                    unit_id: "unit-b".into(),
                },
                UnitOperationData::InsertBefore {
                    unit: local_snapshot("local-x", "inserted", true),
                    before_id: Some("unit-a".into()),
                },
                UnitOperationData::MoveBefore {
                    unit: server_snapshot("unit-a", ""),
                    before_id: None,
                },
            ],
        },
    )
    .await;

    let saved = match saved {
        Ok(saved) => saved,
        Err(_) => panic!("expected save success"),
    };
    let snapshot = mock.snapshot();

    assert_eq!(saved.units.len(), 2);
    assert_eq!(saved.units[0].translated_text.as_deref(), Some("inserted"));
    assert_eq!(saved.units[1].id, "unit-a");
    assert_eq!(saved.local_id_mappings.len(), 1);
    assert_eq!(saved.local_id_mappings[0].local_id, "local-x");
    assert_eq!(saved.total_unit_count, 2);
    assert_eq!(saved.translated_unit_count, 1);
    assert_eq!(saved.proofread_unit_count, 1);
    assert_eq!(snapshot.pages[0].total_unit_count, 2);
    assert_eq!(snapshot.pages[0].translated_unit_count, 1);
    assert_eq!(snapshot.pages[0].proofread_unit_count, 1);
    assert_eq!(snapshot.chapters[0].total_unit_count, 2);
    assert_eq!(snapshot.chapters[0].translated_unit_count, 1);
    assert_eq!(snapshot.chapters[0].proofread_unit_count, 1);
    assert!(snapshot.comics[0].last_active_at > snapshot.comics[0].created_at);
}

#[tokio::test]
async fn save_infos_rolls_back_without_edit_role() {
    let mock = Mock::new();
    seed_scope(&mock, 1, 1, 0);
    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));
    mock.seed_unit(unit("unit-a", 0, "alpha", None, false));

    let err = save_infos(
        &mock,
        &mock,
        token("user-1"),
        SavePageUnitsData {
            page_id: "page-1".into(),
            opers: vec![UnitOperationData::InsertBefore {
                unit: local_snapshot("local-x", "inserted", true),
                before_id: None,
            }],
        },
    )
    .await
    .err()
    .unwrap();
    let snapshot = mock.snapshot();

    match err {
        RootError::Expected { variant, .. } => {
            assert!(matches!(variant, ExpectedVariant::Perm));
        }
        RootError::Unrecoverable { .. } => {
            panic!("expected permission error");
        }
    }
    assert_eq!(snapshot.units.len(), 1);
    assert_eq!(snapshot.pages[0].total_unit_count, 1);
    assert_eq!(snapshot.chapters[0].total_unit_count, 1);
}
