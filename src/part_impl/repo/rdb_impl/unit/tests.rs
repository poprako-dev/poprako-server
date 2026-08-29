// unit_roundtrip_uses_testcontainer(UnitRepo)(positive): batch apply creates, hides, restores, orders, and counts Units.

use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use poprako_rdb_core::RdbCore;

use crate::model::read::proj::unit::UnitOrder;
use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::model::write::unit::UnitEdit;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfos, ListUnitInfosByIds, ListUnitInfosByPageIds,
    ListUnitOrders,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::part_impl::repo::rdb_impl::unit::edit::move_order;
use crate::result::accept;
use crate::util::Patch;

const PREFIX: &str = "rdb-test-unit-domain-";

#[test]
fn move_order_preserves_relative_order_around_the_moved_unit() {
    //
    let mut ordered_ids = vec!["a", "b", "c", "d"];

    move_order(&mut ordered_ids, "d", Some("b")).unwrap();

    assert_eq!(ordered_ids, vec!["a", "d", "b", "c"]);

    move_order(&mut ordered_ids, "d", None).unwrap();

    assert_eq!(ordered_ids, vec!["a", "b", "c", "d"]);
}

/// Verifies Unit v2 persistence through the real transaction adapter.
pub async fn unit_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let page_fixture = test_shared::seed_page(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let first_id = format!("{}first", PREFIX);

    let second_id = format!("{}second", PREFIX);

    let creator_id = page_fixture.chapter_entry.creator_id.clone();

    let create_edits = [
        create_edit(&first_id, &creator_id, "translated"),
        create_edit(&second_id, &creator_id, "second"),
    ];

    let create_orders = [
        UnitOrder {
            id: first_id.clone(),
            next_id: Some(second_id.clone()),
            is_hidden: false,
        },
        UnitOrder {
            id: second_id.clone(),
            next_id: None,
            is_hidden: false,
        },
    ];

    nucl.coord(async |context| {
        //
        let counters = repo
            .step(
                context,
                &ApplyUnitEdits {
                    page_id: &page_fixture.page_entry.id,
                    orders: &[],
                    edits: &create_edits,
                },
            )
            .await?;

        assert_eq!(counters.total, 2);

        accept(())
    })
    .await
    .unwrap();

    let delete_edits = [UnitEdit::Delete {
        id: first_id.clone(),
    }];

    nucl.coord(async |context| {
        //
        let counters = repo
            .step(
                context,
                &ApplyUnitEdits {
                    page_id: &page_fixture.page_entry.id,
                    orders: &create_orders,
                    edits: &delete_edits,
                },
            )
            .await?;

        assert_eq!(counters.total, 1);

        let orders = repo
            .step(
                context,
                &ListUnitOrders {
                    page_id: &page_fixture.page_entry.id,
                },
            )
            .await?;

        assert_eq!(orders.len(), 2);

        accept(())
    })
    .await
    .unwrap();

    let unit_infos = repo
        .run(&ListUnitInfos {
            page_id: &page_fixture.page_entry.id,
        })
        .await
        .unwrap();

    assert_eq!(unit_infos.len(), 2);

    assert!(unit_infos[0].hidden_at.is_some());

    let restore_edits = [UnitEdit::Save {
        id: first_id.clone(),
        next_id: Patch::Clear,
        is_bubble: None,
        coord: None,
        translation: Patch::Skip,
        revision: Patch::Assign {
            value: UnitRevision {
                is_proofread: true,
                proofread_text: Some("proofread".to_string()),
                last_proofreader_id: creator_id,
            },
        },
    }];

    let restore_orders = [
        UnitOrder {
            id: first_id.clone(),
            next_id: Some(second_id.clone()),
            is_hidden: true,
        },
        UnitOrder {
            id: second_id.clone(),
            next_id: None,
            is_hidden: false,
        },
    ];

    nucl.coord(async |context| {
        //
        let counters = repo
            .step(
                context,
                &ApplyUnitEdits {
                    page_id: &page_fixture.page_entry.id,
                    orders: &restore_orders,
                    edits: &restore_edits,
                },
            )
            .await?;

        assert_eq!(counters.total, 2);

        assert_eq!(counters.proofread, 1);

        accept(())
    })
    .await
    .unwrap();

    let unit_infos = repo
        .run(&ListUnitInfos {
            page_id: &page_fixture.page_entry.id,
        })
        .await
        .unwrap();

    assert_eq!(
        unit_infos
            .iter()
            .map(|unit_info| unit_info.id.as_str())
            .collect::<Vec<_>>(),
        vec![second_id.as_str(), first_id.as_str()]
    );

    let page_ids = [
        page_fixture.page_entry.id.clone(),
        format!("{}missing-page", PREFIX),
    ];

    let batch_unit_infos = repo
        .run(&ListUnitInfosByPageIds {
            page_ids: &page_ids,
        })
        .await
        .unwrap();

    assert_eq!(
        batch_unit_infos
            .iter()
            .map(|unit_info| unit_info.id.as_str())
            .collect::<Vec<_>>(),
        vec![second_id.as_str(), first_id.as_str()]
    );

    nucl.coord(async |context| {
        //
        let ids = [first_id.clone(), "missing-unit".to_string()];

        let selected = repo
            .step(context, &ListUnitInfosByIds { ids: &ids })
            .await?;

        assert_eq!(selected.len(), 1);

        assert_eq!(selected[0].id, first_id);

        accept(())
    })
    .await
    .unwrap();

    test_shared::cleanup(&shared, PREFIX).await.unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .unwrap();
}

fn create_edit(id: &str, user_id: &str, text: &str) -> UnitEdit {
    UnitEdit::Create {
        id: id.to_string(),
        next_id: None,
        is_bubble: true,
        coord: UnitCoord {
            x_coord: 1.0,
            y_coord: 2.0,
        },
        translation: Some(UnitTranslation {
            translated_text: text.to_string(),
            last_translator_id: user_id.to_string(),
        }),
        revision: None,
    }
}
