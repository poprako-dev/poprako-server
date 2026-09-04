use super::*;

use crate::model::read::proj::unit::UnitInfo;
use crate::model::shared::unit::UnitCoord;
use crate::result::accept;

#[tokio::test]
async fn list_preserves_visible_order_across_a_tombstone() -> BaseRest<()> {
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![
            create("tail", None, Some("tail")),
            create("hidden", None, Some("hidden")),
            create("head", None, Some("head")),
        ]),
    )
    .await?;

    let snapshot = mock.snapshot();

    let hidden_id = snapshot
        .units
        .iter()
        .find(|unit_info| {
            unit_info.translated_text.as_deref() == Some("hidden")
        })
        .map(|unit_info| unit_info.id.clone())
        .ok_or_else(|| BaseError::Unrecoverable {
            message: "test fixture Unit is missing".into(),
        })?;

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![UnitEditInstr::Delete { id: hidden_id }]),
    )
    .await?;

    let listed = list_infos(
        (&mock, &mock),
        token("translator-1"),
        ListPageUnitInfosInstr {
            page_id: "page-1".to_string(),
        },
    )
    .await?;

    let texts = listed
        .unit_infos
        .iter()
        .filter_map(|unit_info| unit_info.translated_text.as_deref())
        .collect::<Vec<_>>();

    assert_eq!(texts, ["tail", "head"]);

    assert_eq!(listed.total_unit_count, 2);

    accept(())
}

#[tokio::test]
async fn list_rejects_page_count_drift() {
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    mock.seed_unit(unit_info("unit-1"));

    let result = list_infos(
        (&mock, &mock),
        token("translator-1"),
        ListPageUnitInfosInstr {
            page_id: "page-1".to_string(),
        },
    )
    .await;

    assert!(matches!(result, Err(BaseError::Unrecoverable { .. })));
}

fn unit_info(id: &str) -> UnitInfo {
    let current_time = OffsetDateTime::now_utc();

    UnitInfo {
        id: id.to_string(),

        page_id: "page-1".to_string(),
        next_id: None,

        is_bubble: true,

        coord: UnitCoord {
            x_coord: 1.0,
            y_coord: 2.0,
        },

        translated_text: Some("translated".to_string()),
        last_translator_id: Some("translator-1".to_string()),

        is_proofread: false,
        proofread_text: None,
        last_proofreader_id: None,

        hidden_at: None,

        created_at: current_time,
        updated_at: current_time,
    }
}
