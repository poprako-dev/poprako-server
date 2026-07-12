// list_infos(list_infos)(positive): team member lists page units in index order with page counters.
// list_infos(list_infos)(positive): chapter assignee lists page units without team membership.
// list_infos(list_infos)(negative): non-member without assignment cannot list page units.
// save_infos(save_infos)(positive): create maps a local id, save updates, and delete removes.
// save_infos(save_infos)(positive): save with before_id places unit before anchor, None appends to tail.
// save_infos(save_infos)(positive): concurrent merge applies b-then-c and c-then-b to twenty units and reaches consistent final state.
// save_infos(save_infos)(negative): user without edit role rolls back units and counters.
// save_infos(save_infos)(negative): invalid diff rolls back units, counters, and comic activity.
// save_infos(save_infos)(negative): missing text editor ids are rejected before transaction access.

use super::*;

use time::OffsetDateTime;

use crate::data::unit_data;
use crate::model::{
    assignment_model, chapter_model, comic_model, member_model, page_model,
    unit_model, user_model, workset_model,
};
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{ExpectedVariant, RegularError};
use crate::value::chapter::StageMask;
use crate::value::role::{RoleField, RoleMask};

struct TestRng {
    state: u64,
}

impl TestRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        //
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);

        self.state
    }

    fn range(&mut self, bound: usize) -> usize {
        (self.next() as usize) % bound
    }

    fn bool(&mut self) -> bool {
        self.next() & 1 == 1
    }
}

fn token(user_id: &str) -> user_model::Token {
    user_model::Token {
        user_id: user_id.into(),
    }
}

fn workset(id: &str, team_id: &str) -> workset_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    workset_model::Info {
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

fn comic(id: &str, workset_id: &str) -> comic_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    comic_model::Info {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 0,
        title: "comic".into(),
        author: "author".into(),
        description: None,
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

fn chapter(
    id: &str,
    comic_id: &str,
    total: i32,
    translated: i32,
    proofread: i32,
) -> chapter_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    chapter_model::Info {
        id: id.into(),
        comic_id: comic_id.into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "chapter".into(),
        page_count: 1,
        total_unit_count: total,
        translated_unit_count: translated,
        proofread_unit_count: proofread,
        stages: StageMask::try_from(0u32).ok().unwrap(),
        creator_id: "user-1".into(),
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

fn member(user_id: &str) -> member_model::Info {
    member_model::Info {
        id: format!("member-{}", user_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: "team-1".into(),
        user: None,
        team: None,
        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> assignment_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    assignment_model::Info {
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

fn page(
    id: &str,
    total: i32,
    translated: i32,
    proofread: i32,
) -> page_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    page_model::Info {
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
) -> unit_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    unit_model::Info {
        id: id.into(),
        page_id: page_id.into(),
        index,
        is_bubble: true,
        is_proofread: proofread,
        x_coord: 1.0,
        y_coord: 2.0,
        translated_text: Some(text.into()),
        last_translator_id: None,
        proofread_text: proofread_text.map(Into::into),
        last_proofreader_id: None,
        created_at: time,
        updated_at: time,
    }
}

fn create_oper(
    local_id: &str,
    text: &str,
    before_id: Option<&str>,
) -> unit_data::OperData {
    unit_data::OperData::Create {
        local_id: local_id.into(),
        before_id: before_id.map(Into::into),
        is_bubble: true,
        is_proofread: false,
        x_coord: 3.0,
        y_coord: 4.0,
        translated_text: Some(text.into()),
        last_translator_id: Some("user-1".into()),
        proofread_text: None,
        last_proofreader_id: None,
    }
}

fn save_oper(
    id: &str,
    text: &str,
    before_id: Option<&str>,
) -> unit_data::OperData {
    save_oper_with_payload(id, text, false, 5.0, 6.0, before_id)
}

fn delete_oper(id: &str) -> unit_data::OperData {
    unit_data::OperData::Delete { id: id.into() }
}

fn save_oper_with_payload(
    id: &str,
    text: &str,
    proofread: bool,
    x_coord: f64,
    y_coord: f64,
    before_id: Option<&str>,
) -> unit_data::OperData {
    unit_data::OperData::Save {
        id: id.into(),
        before_id: before_id.map(Into::into),
        is_bubble: true,
        is_proofread: proofread,
        x_coord,
        y_coord,
        translated_text: Some(text.into()),
        last_translator_id: Some("user-1".into()),
        proofread_text: None,
        last_proofreader_id: None,
    }
}

fn seed_scope(mock: &Mock, total: i32, translated: i32, proofread: i32) {
    //
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

fn sorted_unit_ids(units: &[unit_model::Info]) -> Vec<String> {
    //
    let mut unit_infos = units.to_vec();

    unit_infos.sort_by_key(|left| left.index);

    unit_infos
        .into_iter()
        .map(|unit_info| unit_info.id)
        .collect()
}

fn assert_perm_error(error: RegularError) {
    match error {
        //
        RegularError::Expected { variant, .. } => {
            assert!(matches!(variant, ExpectedVariant::Perm));
        }

        RegularError::Unrecoverable { .. } => {
            panic!("expected permission error");
        }
    }
}

fn assert_args_error(error: RegularError) {
    match error {
        //
        RegularError::Expected { variant, .. } => {
            assert!(matches!(variant, ExpectedVariant::Args));
        }

        RegularError::Unrecoverable { .. } => {
            panic!("expected argument error");
        }
    }
}

#[tokio::test]
async fn list_infos_returns_units_for_team_member() {
    //
    let mock = Mock::new();

    seed_scope(&mock, 2, 1, 1);

    mock.seed_member(member("user-1"));

    mock.seed_unit(unit("unit-b", "page-1", 1, "beta", None, false));

    mock.seed_unit(unit("unit-a", "page-1", 0, "", Some("proof"), true));

    let listed = list_infos(
        &mock,
        token("user-1"),
        unit_data::ListPageInfosData {
            page_id: "page-1".into(),
            offset: 0,
            limit: 100,
        },
    )
    .await;

    let listed = match listed {
        //
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
    //
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
        unit_data::ListPageInfosData {
            page_id: "page-1".into(),
            offset: 0,
            limit: 100,
        },
    )
    .await;

    let listed = match listed {
        //
        Ok(listed) => listed,

        Err(_) => panic!("expected list success"),
    };

    assert_eq!(listed.unit_infos.len(), 1);

    assert_eq!(listed.unit_infos[0].id, "unit-a");
}

#[tokio::test]
async fn list_infos_rejects_unrelated_user() {
    //
    let mock = Mock::new();

    seed_scope(&mock, 0, 0, 0);

    let e = list_infos(
        &mock,
        token("user-2"),
        unit_data::ListPageInfosData {
            page_id: "page-1".into(),
            offset: 0,
            limit: 100,
        },
    )
    .await
    .err()
    .unwrap();

    assert_perm_error(e);
}

#[tokio::test]
async fn save_infos_creates_updates_and_deletes_by_typed_opers() {
    //
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
        unit_data::SavePageData {
            page_id: "page-1".into(),
            diff: unit_data::DiffData {
                page_id: "page-1".into(),
                opers: vec![
                    delete_oper("unit-b"),
                    create_oper("local-x", "inserted", Some("unit-a")),
                    save_oper("unit-a", "alpha-v2", None),
                ],
            },
        },
    )
    .await;

    let saved = match saved {
        //
        Ok(saved) => saved,

        Err(_) => panic!("expected save success"),
    };

    let snapshot = mock.snapshot();

    let created_id = saved.local_id_mappers[0].unit_id.clone();

    assert_eq!(saved.local_id_mappers.len(), 1);

    assert_eq!(saved.local_id_mappers[0].local_id, "local-x");

    assert_eq!(saved.total_unit_count, 2);

    assert_eq!(
        sorted_unit_ids(&snapshot.units),
        vec![created_id, "unit-a".into()]
    );

    assert_eq!(snapshot.pages[0].total_unit_count, 2);

    assert_eq!(snapshot.chapters[0].total_unit_count, 2);

    assert!(snapshot.comics[0].last_active_at > snapshot.comics[0].created_at);
}

#[tokio::test]
async fn save_infos_places_unit_before_anchor_or_at_tail_by_before_id() {
    //
    let mock = Mock::new();

    seed_scope(&mock, 2, 2, 0);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock.seed_unit(unit("unit-a", "page-1", 0, "alpha", None, false));

    mock.seed_unit(unit("unit-b", "page-1", 1, "beta", None, false));

    let saved = save_infos(
        &mock,
        &mock,
        token("user-1"),
        unit_data::SavePageData {
            page_id: "page-1".into(),
            diff: unit_data::DiffData {
                page_id: "page-1".into(),
                opers: vec![
                    save_oper("unit-c", "gamma", Some("unit-a")),
                    save_oper("unit-d", "delta", None),
                ],
            },
        },
    )
    .await;

    let saved = match saved {
        //
        Ok(saved) => saved,

        Err(_) => panic!("expected save success"),
    };

    let snapshot = mock.snapshot();

    assert_eq!(saved.local_id_mappers.len(), 0);

    assert_eq!(saved.total_unit_count, 4);

    assert_eq!(
        sorted_unit_ids(&snapshot.units),
        vec!["unit-c", "unit-a", "unit-b", "unit-d"]
    );
}

fn oracle_ids(units: &[OracleUnit]) -> Vec<String> {
    units.iter().map(|unit| unit.id.clone()).collect()
}

struct OracleUnit {
    id: String,
    text: String,
    proofread: bool,
    x_coord: f64,
    y_coord: f64,
}

fn clone_oracle(units: &[OracleUnit]) -> Vec<OracleUnit> {
    units
        .iter()
        .map(|unit| OracleUnit {
            id: unit.id.clone(),
            text: unit.text.clone(),
            proofread: unit.proofread,
            x_coord: unit.x_coord,
            y_coord: unit.y_coord,
        })
        .collect()
}

#[tokio::test]
async fn save_infos_concurrent_merge_reaches_consistent_final_state() {
    //
    let initial_count = 20;

    let oper_count = 20;

    let mut rng = TestRng::new(0x5eed_2026);

    let mut initial_units = Vec::new();

    for index in 0..initial_count {
        //
        let unit_id = format!("unit-{}", index);

        initial_units.push(OracleUnit {
            id: unit_id.clone(),
            text: format!("initial-text-{}", index),
            proofread: index % 3 == 0,
            x_coord: 1.0,
            y_coord: 2.0,
        });
    }

    let b_opers =
        generate_random_opers(&mut rng, &initial_units, "b", oper_count);

    let c_opers =
        generate_random_opers(&mut rng, &initial_units, "c", oper_count);

    let mut b_then_c_oracle = clone_oracle(&initial_units);

    apply_opers_to_oracle(&mut b_then_c_oracle, &b_opers, "b");

    apply_opers_to_oracle(&mut b_then_c_oracle, &c_opers, "c");

    let mut c_then_b_oracle = clone_oracle(&initial_units);

    apply_opers_to_oracle(&mut c_then_b_oracle, &c_opers, "c");

    apply_opers_to_oracle(&mut c_then_b_oracle, &b_opers, "b");

    let b_then_c_mock = build_seeded_mock(&initial_units);

    let c_then_b_mock = build_seeded_mock(&initial_units);

    assert!(
        apply_save_to_mock(&b_then_c_mock, &b_opers).await.is_ok(),
        "b-then-c first save failed",
    );

    assert!(
        apply_save_to_mock(&c_then_b_mock, &c_opers).await.is_ok(),
        "c-then-b first save failed",
    );

    assert!(
        apply_save_to_mock(&b_then_c_mock, &c_opers).await.is_ok(),
        "b-then-c second save failed",
    );

    assert!(
        apply_save_to_mock(&c_then_b_mock, &b_opers).await.is_ok(),
        "c-then-b second save failed",
    );

    let b_then_c_snapshot = b_then_c_mock.snapshot();

    let c_then_b_snapshot = c_then_b_mock.snapshot();

    let b_then_c_actual =
        collect_sorted_units(&b_then_c_snapshot.units, "page-1");

    let c_then_b_actual =
        collect_sorted_units(&c_then_b_snapshot.units, "page-1");

    assert_eq!(
        b_then_c_actual
            .iter()
            .map(|u| u.id.clone())
            .collect::<Vec<_>>(),
        oracle_ids(&b_then_c_oracle),
        "b-then-c final order must match oracle"
    );

    assert_eq!(
        c_then_b_actual
            .iter()
            .map(|u| u.id.clone())
            .collect::<Vec<_>>(),
        oracle_ids(&c_then_b_oracle),
        "c-then-b final order must match oracle"
    );

    assert_eq!(
        b_then_c_actual
            .iter()
            .map(|u| u.translated_text.clone())
            .collect::<Vec<_>>(),
        b_then_c_oracle
            .iter()
            .map(|u| Some(u.text.clone()))
            .collect::<Vec<_>>(),
        "b-then-c final text must match oracle"
    );

    assert_eq!(
        c_then_b_actual
            .iter()
            .map(|u| u.translated_text.clone())
            .collect::<Vec<_>>(),
        c_then_b_oracle
            .iter()
            .map(|u| Some(u.text.clone()))
            .collect::<Vec<_>>(),
        "c-then-b final text must match oracle"
    );

    assert_eq!(
        b_then_c_actual
            .iter()
            .map(|u| u.is_proofread)
            .collect::<Vec<_>>(),
        b_then_c_oracle
            .iter()
            .map(|u| u.proofread)
            .collect::<Vec<_>>(),
        "b-then-c proofread flags must match oracle"
    );

    assert_eq!(
        c_then_b_actual
            .iter()
            .map(|u| u.is_proofread)
            .collect::<Vec<_>>(),
        c_then_b_oracle
            .iter()
            .map(|u| u.proofread)
            .collect::<Vec<_>>(),
        "c-then-b proofread flags must match oracle"
    );

    assert_eq!(
        b_then_c_actual.len(),
        c_then_b_actual.len(),
        "both merge orders must reach the same unit count"
    );
}

fn generate_random_opers(
    rng: &mut TestRng,
    initial_units: &[OracleUnit],
    tag: &str,
    count: usize,
) -> Vec<unit_data::OperData> {
    //
    let mut opers = Vec::with_capacity(count);

    for step in 0..count {
        //
        let subject_index = rng.range(initial_units.len());

        let subject_id = initial_units[subject_index].id.clone();

        let before_id = if rng.bool() {
            Some(initial_units[rng.range(initial_units.len())].id.clone())
        } else {
            None
        };

        let text = format!("text-{}-{}-{}", tag, subject_id, step);

        let proofread = rng.bool();

        opers.push(save_oper_with_payload(
            &subject_id,
            &text,
            proofread,
            10.0 + step as f64,
            20.0 + step as f64,
            before_id.as_deref(),
        ));
    }

    opers
}

fn apply_opers_to_oracle(
    oracle_units: &mut Vec<OracleUnit>,
    opers: &[unit_data::OperData],
    tag: &str,
) {
    for (step, oper) in opers.iter().enumerate() {
        match oper {
            //
            unit_data::OperData::Create {
                local_id,
                before_id,
                is_proofread,
                x_coord,
                y_coord,
                ..
            } => {
                //
                let resolved_id = local_id.clone();

                let oracle_unit = OracleUnit {
                    id: resolved_id.clone(),
                    text: format!("text-{}-{}-{}", tag, resolved_id, step),
                    proofread: *is_proofread,
                    x_coord: *x_coord,
                    y_coord: *y_coord,
                };

                oracle_units.retain(|unit| unit.id != resolved_id);

                let insert_position = before_id
                    .as_ref()
                    .filter(|before_id| *before_id != &resolved_id)
                    .and_then(|before_id| {
                        oracle_units
                            .iter()
                            .position(|unit| unit.id == *before_id)
                    })
                    .unwrap_or(oracle_units.len());

                oracle_units.insert(insert_position, oracle_unit);
            }

            //
            unit_data::OperData::Save {
                id,
                before_id,
                is_proofread,
                x_coord,
                y_coord,
                ..
            } => {
                //
                let resolved_id = id.clone();

                let oracle_unit = OracleUnit {
                    id: resolved_id.clone(),
                    text: format!("text-{}-{}-{}", tag, resolved_id, step),
                    proofread: *is_proofread,
                    x_coord: *x_coord,
                    y_coord: *y_coord,
                };

                oracle_units.retain(|unit| unit.id != resolved_id);

                let insert_position = before_id
                    .as_ref()
                    .filter(|before_id| *before_id != &resolved_id)
                    .and_then(|before_id| {
                        oracle_units
                            .iter()
                            .position(|unit| unit.id == *before_id)
                    })
                    .unwrap_or(oracle_units.len());

                oracle_units.insert(insert_position, oracle_unit);
            }

            unit_data::OperData::Delete { id } => {
                oracle_units.retain(|unit| unit.id != *id);
            }
        }
    }
}

fn build_seeded_mock(initial_units: &[OracleUnit]) -> Mock {
    //
    let mock = Mock::new();

    let initial_count = initial_units.len() as i32;

    seed_scope(&mock, initial_count, initial_count, 0);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    for (index, oracle_unit) in initial_units.iter().enumerate() {
        mock.seed_unit(unit(
            &oracle_unit.id,
            "page-1",
            index as i32,
            &oracle_unit.text,
            None,
            oracle_unit.proofread,
        ));
    }

    mock
}

async fn apply_save_to_mock(
    mock: &Mock,
    opers: &[unit_data::OperData],
) -> RegularResult<()> {
    //
    save_infos(
        mock,
        mock,
        token("user-1"),
        unit_data::SavePageData {
            page_id: "page-1".into(),
            diff: unit_data::DiffData {
                page_id: "page-1".into(),
                opers: opers.to_vec(),
            },
        },
    )
    .await?;

    Ok(())
}

fn collect_sorted_units(
    units: &[unit_model::Info],
    page_id: &str,
) -> Vec<unit_model::Info> {
    //
    let mut filtered: Vec<unit_model::Info> = units
        .iter()
        .filter(|unit_info| unit_info.page_id == page_id)
        .cloned()
        .collect();

    filtered.sort_by_key(|unit_info| unit_info.index);

    filtered
}

#[tokio::test]
async fn save_infos_rolls_back_without_edit_role() {
    //
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
        unit_data::SavePageData {
            page_id: "page-1".into(),
            diff: unit_data::DiffData {
                page_id: "page-1".into(),
                opers: vec![create_oper("local-x", "inserted", None)],
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
    //
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
        unit_data::SavePageData {
            page_id: "page-1".into(),
            diff: unit_data::DiffData {
                page_id: "page-1".into(),
                opers: vec![
                    create_oper("local-x", "inserted", None),
                    create_oper("local-x", "duplicate", None),
                ],
            },
        },
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_args_error(e);

    assert_eq!(snapshot.units[0].translated_text.as_deref(), Some("alpha"));

    assert_eq!(snapshot.pages[0].total_unit_count, 1);

    assert_eq!(snapshot.chapters[0].total_unit_count, 1);

    assert_eq!(
        snapshot.comics[0].last_active_at,
        before_snapshot.comics[0].last_active_at
    );
}

#[tokio::test]
async fn save_infos_rejects_missing_text_editor_ids_before_transaction() {
    //
    let mock = Mock::new();

    let create_error = save_infos(
        &mock,
        &mock,
        token("user-1"),
        unit_data::SavePageData {
            page_id: "page-1".into(),
            diff: unit_data::DiffData {
                page_id: "page-1".into(),
                opers: vec![unit_data::OperData::Create {
                    local_id: "local-1".into(),
                    before_id: None,
                    is_bubble: true,
                    is_proofread: false,
                    x_coord: 1.0,
                    y_coord: 2.0,
                    translated_text: Some("translated".into()),
                    last_translator_id: None,
                    proofread_text: None,
                    last_proofreader_id: None,
                }],
            },
        },
    )
    .await
    .err()
    .unwrap();

    assert_args_error(create_error);

    let save_error = save_infos(
        &mock,
        &mock,
        token("user-1"),
        unit_data::SavePageData {
            page_id: "page-1".into(),
            diff: unit_data::DiffData {
                page_id: "page-1".into(),
                opers: vec![unit_data::OperData::Save {
                    id: "unit-a".into(),
                    before_id: None,
                    is_bubble: true,
                    is_proofread: true,
                    x_coord: 1.0,
                    y_coord: 2.0,
                    translated_text: None,
                    last_translator_id: None,
                    proofread_text: Some("proofread".into()),
                    last_proofreader_id: Some(String::new()),
                }],
            },
        },
    )
    .await
    .err()
    .unwrap();

    assert_args_error(save_error);

    assert!(mock.snapshot().units.is_empty());
}
