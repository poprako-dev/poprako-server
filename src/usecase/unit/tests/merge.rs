use super::*;

use crate::data::unit::{SavePageUnitsParams, UnitDiffParams, UnitOperParams};
use crate::model::unit::UnitInfo;

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
) -> Vec<UnitOperParams> {
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
    opers: &[UnitOperParams],
    tag: &str,
) {
    for (step, oper) in opers.iter().enumerate() {
        match oper {
            //
            UnitOperParams::Create {
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
            UnitOperParams::Save {
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

            UnitOperParams::Delete { id } => {
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
    opers: &[UnitOperParams],
) -> BaseResult<()> {
    //
    save_infos(
        mock,
        mock,
        token("user-1"),
        SavePageUnitsParams {
            page_id: "page-1".into(),
            diff: UnitDiffParams {
                page_id: "page-1".into(),
                opers: opers.to_vec(),
            },
        },
    )
    .await?;

    accept(())
}

fn collect_sorted_units(units: &[UnitInfo], page_id: &str) -> Vec<UnitInfo> {
    //
    let mut filtered: Vec<UnitInfo> = units
        .iter()
        .filter(|unit_info| unit_info.page_id == page_id)
        .cloned()
        .collect();

    filtered.sort_by_key(|unit_info| unit_info.index);

    filtered
}
