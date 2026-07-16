use super::*;

use crate::data::unit::{SavePageUnitsParams, UnitDiffParams, UnitOperParams};

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

    let e = save(
        &mock,
        &mock,
        token("user-1"),
        SavePageUnitsParams {
            page_id: "page-1".into(),
            diff: UnitDiffParams {
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

    let e = save(
        &mock,
        &mock,
        token("user-1"),
        SavePageUnitsParams {
            page_id: "page-1".into(),
            diff: UnitDiffParams {
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

    let create_error = save(
        &mock,
        &mock,
        token("user-1"),
        SavePageUnitsParams {
            page_id: "page-1".into(),
            diff: UnitDiffParams {
                page_id: "page-1".into(),
                opers: vec![UnitOperParams::Create {
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

    let save_error = save(
        &mock,
        &mock,
        token("user-1"),
        SavePageUnitsParams {
            page_id: "page-1".into(),
            diff: UnitDiffParams {
                page_id: "page-1".into(),
                opers: vec![UnitOperParams::Save {
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
