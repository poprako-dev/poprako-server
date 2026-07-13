use super::*;

use crate::data::unit::{ListPageUnitInfosParams, SavePageUnitsParams, UnitDiffParams};

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
        ListPageUnitInfosParams {
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
        ListPageUnitInfosParams {
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
        ListPageUnitInfosParams {
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
        SavePageUnitsParams {
            page_id: "page-1".into(),
            diff: UnitDiffParams {
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
        SavePageUnitsParams {
            page_id: "page-1".into(),
            diff: UnitDiffParams {
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
