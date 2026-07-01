// list_infos(list_infos)(positive): team member lists page units in index order with page counters.
// list_infos(list_infos)(positive): chapter assignee lists page units without team membership.
// list_infos(list_infos)(negative): non-member without assignment cannot list page units.
// save_infos(save_infos)(positive): translator saves create, save, delete, and order diff with mappings and counters only.
// save_infos(save_infos)(positive): proofreader restores a stale missing unit by save upsert.
// save_infos(save_infos)(negative): user without edit role rolls back units and counters.
// save_infos(save_infos)(negative): invalid diff rolls back units, counters, and comic activity.

use super::*;

use time::OffsetDateTime;

use crate::data::unit::{ListPageUnitInfosData, SavePageUnitsData, UnitDiffData, UnitOperData};
use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::member::MemberInfo;
use crate::model::page::PageInfo;
use crate::model::unit::UnitInfo;
use crate::model::user::UserToken;
use crate::model::workset::WorksetInfo;
use crate::part_impl::repo_mock::Mock;
use crate::result::{ExpectedVariant, RegularError};
use crate::value::chapter::WorkflowStageMask;
use crate::value::role::{RoleField, RoleMask};

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
        workset: None,
        team: None,
        creator: None,
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
        creator: None,
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
        user: None,
        team: None,
        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn assignment(chapter_id: &str, user_id: &str, role_mask: RoleMask) -> AssignmentInfo {
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: format!("assignment-{}-{}", chapter_id, user_id),
        chapter_id: chapter_id.into(),
        user_id: user_id.into(),
        user: None,
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
    page_id: &str,
    index: i32,
    text: &str,
    proofread_text: Option<&str>,
    proofread: bool,
) -> UnitInfo {
    let time = OffsetDateTime::now_utc();

    UnitInfo {
        id: id.into(),
        page_id: page_id.into(),
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

fn create_oper(local_id: &str, text: &str, proofread: bool) -> UnitOperData {
    UnitOperData {
        id: None,
        local_id: Some(local_id.into()),
        is_bubble: Some(true),
        is_proofread: Some(proofread),
        x_coord: Some(3.0),
        y_coord: Some(4.0),
        translated_text: Some(text.into()),
        translator_comment: None,
        last_translator_id: None,
        proofread_text: None,
        proofreader_comment: None,
        last_proofreader_id: None,
    }
}

fn save_oper(id: &str, text: &str, proofread: bool) -> UnitOperData {
    UnitOperData {
        id: Some(id.into()),
        local_id: None,
        is_bubble: Some(true),
        is_proofread: Some(proofread),
        x_coord: Some(5.0),
        y_coord: Some(6.0),
        translated_text: Some(text.into()),
        translator_comment: None,
        last_translator_id: None,
        proofread_text: None,
        proofreader_comment: None,
        last_proofreader_id: None,
    }
}

fn delete_oper(id: &str) -> UnitOperData {
    UnitOperData {
        id: Some(id.into()),
        local_id: None,
        is_bubble: None,
        is_proofread: None,
        x_coord: None,
        y_coord: None,
        translated_text: None,
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

fn sorted_unit_ids(units: &[UnitInfo]) -> Vec<String> {
    let mut unit_infos = units.to_vec();

    unit_infos.sort_by(|left, right| left.index.cmp(&right.index));

    unit_infos
        .into_iter()
        .map(|unit_info| unit_info.id)
        .collect()
}

fn assert_perm_error(error: RegularError) {
    match error {
        RegularError::Expected { variant, .. } => {
            assert!(matches!(variant, ExpectedVariant::PermDeny));
        }
        RegularError::Unrecoverable { .. } => {
            panic!("expected permission error");
        }
    }
}

#[tokio::test]
async fn list_infos_returns_units_for_team_member() {
    let mock = Mock::new();

    seed_scope(&mock, 2, 1, 1);

    mock.seed_member(member("user-1"));

    mock.seed_unit(unit("unit-b", "page-1", 1, "beta", None, false));

    mock.seed_unit(unit("unit-a", "page-1", 0, "", Some("proof"), true));

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

    assert_eq!(listed.unit_infos.len(), 2);

    assert_eq!(listed.unit_infos[0].id, "unit-a");

    assert_eq!(listed.unit_infos[1].id, "unit-b");

    assert_eq!(listed.total_unit_count, 2);

    assert_eq!(listed.translated_unit_count, 1);

    assert_eq!(listed.proofread_unit_count, 1);
}

#[tokio::test]
async fn list_infos_returns_units_for_assignment_fallback() {
    let mock = Mock::new();

    seed_scope(&mock, 1, 1, 0);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-2",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_unit(unit("unit-a", "page-1", 0, "alpha", None, false));

    let listed = list_infos(
        &mock,
        token("user-2"),
        ListPageUnitInfosData {
            page_id: "page-1".into(),
        },
    )
    .await;

    let listed = match listed {
        Ok(listed) => listed,
        Err(_) => panic!("expected list success"),
    };

    assert_eq!(listed.unit_infos.len(), 1);

    assert_eq!(listed.unit_infos[0].id, "unit-a");
}

#[tokio::test]
async fn list_infos_rejects_unrelated_user() {
    let mock = Mock::new();

    seed_scope(&mock, 0, 0, 0);

    let e = list_infos(
        &mock,
        token("user-2"),
        ListPageUnitInfosData {
            page_id: "page-1".into(),
        },
    )
    .await
    .err()
    .unwrap();

    assert_perm_error(e);
}

#[tokio::test]
async fn save_infos_applies_compact_diff_without_returning_units() {
    let mock = Mock::new();

    seed_scope(&mock, 2, 2, 1);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_unit(unit("unit-a", "page-1", 0, "alpha", None, false));

    mock.seed_unit(unit("unit-b", "page-1", 1, "beta", Some("proof"), true));

    let saved = save_infos(
        &mock,
        &mock,
        token("user-1"),
        SavePageUnitsData {
            page_id: "page-1".into(),
            diff: UnitDiffData {
                page_id: "page-1".into(),
                opers: vec![
                    delete_oper("unit-b"),
                    create_oper("local-x", "inserted", true),
                    save_oper("unit-a", "", false),
                ],
                candidate_order: vec!["local-x".into(), "unit-a".into()],
            },
        },
    )
    .await;

    let saved = match saved {
        Ok(saved) => saved,
        Err(_) => panic!("expected save success"),
    };

    let snapshot = mock.snapshot();

    let created_id = saved.local_id_mappers[0].unit_id.clone();

    assert_eq!(saved.local_id_mappers.len(), 1);

    assert_eq!(saved.local_id_mappers[0].local_id, "local-x");

    assert_eq!(saved.total_unit_count, 2);

    assert_eq!(saved.translated_unit_count, 1);

    assert_eq!(saved.proofread_unit_count, 1);

    assert_eq!(
        sorted_unit_ids(&snapshot.units),
        vec![created_id, "unit-a".into()]
    );

    assert_eq!(snapshot.pages[0].total_unit_count, 2);

    assert_eq!(snapshot.pages[0].translated_unit_count, 1);

    assert_eq!(snapshot.pages[0].proofread_unit_count, 1);

    assert_eq!(snapshot.chapters[0].total_unit_count, 2);

    assert_eq!(snapshot.chapters[0].translated_unit_count, 1);

    assert_eq!(snapshot.chapters[0].proofread_unit_count, 1);

    assert!(snapshot.comics[0].last_active_at > snapshot.comics[0].created_at);
}

#[tokio::test]
async fn save_infos_restores_stale_missing_unit_by_save_upsert() {
    let mock = Mock::new();

    seed_scope(&mock, 2, 2, 0);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::PROOFREADER),
    ));

    mock.seed_unit(unit("unit-a", "page-1", 0, "alpha", None, false));

    mock.seed_unit(unit("unit-c", "page-1", 1, "gamma", None, false));

    let saved = save_infos(
        &mock,
        &mock,
        token("user-1"),
        SavePageUnitsData {
            page_id: "page-1".into(),
            diff: UnitDiffData {
                page_id: "page-1".into(),
                opers: vec![save_oper("unit-b", "restored", false)],
                candidate_order: vec!["unit-c".into(), "unit-b".into(), "unit-a".into()],
            },
        },
    )
    .await;

    let saved = match saved {
        Ok(saved) => saved,
        Err(_) => panic!("expected save success"),
    };

    let snapshot = mock.snapshot();

    assert_eq!(saved.local_id_mappers.len(), 0);

    assert_eq!(saved.total_unit_count, 3);

    assert_eq!(saved.translated_unit_count, 3);

    assert_eq!(saved.proofread_unit_count, 0);

    assert_eq!(
        sorted_unit_ids(&snapshot.units),
        vec!["unit-c", "unit-b", "unit-a"]
    );
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

    mock.seed_unit(unit("unit-a", "page-1", 0, "alpha", None, false));

    let e = save_infos(
        &mock,
        &mock,
        token("user-1"),
        SavePageUnitsData {
            page_id: "page-1".into(),
            diff: UnitDiffData {
                page_id: "page-1".into(),
                opers: vec![create_oper("local-x", "inserted", true)],
                candidate_order: vec!["local-x".into(), "unit-a".into()],
            },
        },
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_perm_error(e);

    assert_eq!(snapshot.units.len(), 1);

    assert_eq!(snapshot.pages[0].total_unit_count, 1);

    assert_eq!(snapshot.chapters[0].total_unit_count, 1);
}

#[tokio::test]
async fn save_infos_rolls_back_invalid_diff() {
    let mock = Mock::new();

    seed_scope(&mock, 1, 1, 0);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_unit(unit("unit-a", "page-1", 0, "alpha", None, false));

    let before_snapshot = mock.snapshot();

    let e = save_infos(
        &mock,
        &mock,
        token("user-1"),
        SavePageUnitsData {
            page_id: "page-1".into(),
            diff: UnitDiffData {
                page_id: "page-1".into(),
                opers: vec![save_oper("unit-a", "changed", false)],
                candidate_order: Vec::new(),
            },
        },
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    match e {
        RegularError::Expected { variant, .. } => {
            assert!(matches!(variant, ExpectedVariant::ArgsInvalid));
        }
        RegularError::Unrecoverable { .. } => {
            panic!("expected argument error");
        }
    }

    assert_eq!(snapshot.units[0].translated_text.as_deref(), Some("alpha"));

    assert_eq!(snapshot.pages[0].total_unit_count, 1);

    assert_eq!(snapshot.chapters[0].total_unit_count, 1);

    assert_eq!(
        snapshot.comics[0].last_active_at,
        before_snapshot.comics[0].last_active_at
    );
}
