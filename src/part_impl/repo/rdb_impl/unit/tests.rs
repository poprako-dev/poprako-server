#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// unit_roundtrip_uses_testcontainer(UnitRepo)(positive): batch apply creates, hides, restores, orders, and counts Units.

mod regression;
// Review flag persistence and page aggregation.
mod flagged;

use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use poprako_rdb_core::RdbCore;

use crate::model::read::proj::unit::UnitOrder;
use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::model::write::unit::UnitEdit;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfosByIds, ListUnitInfosByPageIds, ListUnitOrders,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::part_impl::repo::rdb_impl::test_shared::PageFixture;
use crate::result::{BaseError, accept};
use crate::util::Patch;

const PREFIX: &str = "rdb-test-unit-domain-";

// Verify batched page reads and selected unit fields after restoration.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn verify_selected_units(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
    (first_id, second_id): (&str, &str),
) {
    let missing_page_id = format!("{}missing-page", PREFIX);
    let page_ids = [
        page_fixture.page_entry.id.as_str(),
        missing_page_id.as_str(),
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
        vec![second_id, first_id]
    );

    nucl.coord(async |context| {
        //
        let ids = [first_id, "missing-unit"];

        let selected = repo
            .step(context, &ListUnitInfosByIds { ids: &ids })
            .await?;

        assert_eq!(selected.len(), 1);

        assert_eq!(
            selected.as_slice().first().map(|unit| unit.id.as_str()),
            Some(first_id),
        );

        assert!(
            selected
                .as_slice()
                .first()
                .is_some_and(|unit| !unit.is_bubble)
        );

        assert_eq!(
            selected.as_slice().first().map(|unit| unit.coord.x_coord),
            Some(3.0)
        );

        assert_eq!(
            selected.as_slice().first().map(|unit| unit.coord.y_coord),
            Some(4.0)
        );

        assert!(
            selected
                .as_slice()
                .first()
                .is_some_and(|unit| unit.translated_text.is_none()),
        );

        assert!(selected.as_slice().first().is_some_and(|unit| {
            unit.proofread_text.as_deref() == Some("proofread")
        }),);

        accept(())
    })
    .await
    .unwrap();
}

// Restore the hidden unit through a proofread save and verify restored counters.
async fn verify_restore(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
    (first_id, second_id): (&str, &str),
) {
    let creator_id = page_fixture.chapter_entry.creator_id.clone().into_owned();

    let restore_edits = [UnitEdit::Save {
        id: first_id.to_owned(),
        next_id: Patch::Clear,
        is_bubble: None,
        is_flagged: None,
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
            id: first_id.to_owned(),
            next_id: Some(second_id.to_owned()),
            is_hidden: true,
        },
        UnitOrder {
            id: second_id.to_owned(),
            next_id: None,
            is_hidden: false,
        },
    ];

    nucl.coord(async |context| {
        //
        let count_metrics = repo
            .step(
                context,
                &ApplyUnitEdits {
                    page_id: &page_fixture.page_entry.id,
                    orders: &restore_orders,
                    edits: &restore_edits,
                },
            )
            .await?;

        assert_eq!(count_metrics.total, 2);

        assert_eq!(count_metrics.proofread, 1);

        accept(())
    })
    .await
    .unwrap();
}

// Hide one unit while keeping its historical order record.
async fn verify_tombstone(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
    first_id: &str,
    create_orders: &[UnitOrder],
) {
    let delete_edits = [UnitEdit::Delete {
        id: first_id.to_owned(),
    }];

    nucl.coord(async |context| {
        //
        let count_metrics = repo
            .step(
                context,
                &ApplyUnitEdits {
                    page_id: &page_fixture.page_entry.id,
                    orders: create_orders,
                    edits: &delete_edits,
                },
            )
            .await?;

        assert_eq!(count_metrics.total, 1);

        let orders = repo
            .step(
                context,
                &ListUnitOrders {
                    page_id: &page_fixture.page_entry.id,
                },
            )
            .await?;

        assert_eq!(orders.len(), 2);

        assert!(
            orders
                .as_slice()
                .first()
                .is_some_and(|order| order.is_hidden)
        );

        accept(())
    })
    .await
    .unwrap();
}

// Apply mixed field patches and verify whitespace translations do not inflate counters.
async fn verify_mixed_patches(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
    (first_id, second_id): (&str, &str),
) {
    let mixed_patch_edits = [
        UnitEdit::Save {
            id: first_id.to_owned(),
            next_id: Patch::Assign {
                value: second_id.to_owned(),
            },
            is_bubble: Some(false),
            is_flagged: None,
            coord: Some(UnitCoord {
                x_coord: 3.0,
                y_coord: 4.0,
            }),
            translation: Patch::Clear,
            revision: Patch::Clear,
        },
        UnitEdit::Save {
            id: second_id.to_owned(),
            next_id: Patch::Skip,
            is_bubble: None,
            is_flagged: None,
            coord: None,
            translation: Patch::Assign {
                value: UnitTranslation {
                    translated_text: "\u{3000}".to_string(),
                    last_translator_id: page_fixture
                        .chapter_entry
                        .creator_id
                        .clone()
                        .into_owned(),
                },
            },
            revision: Patch::Skip,
        },
    ];

    nucl.coord(async |context| {
        //
        let orders = repo
            .step(
                context,
                &ListUnitOrders {
                    page_id: &page_fixture.page_entry.id,
                },
            )
            .await?;

        let count_metrics = repo
            .step(
                context,
                &ApplyUnitEdits {
                    page_id: &page_fixture.page_entry.id,
                    orders: &orders,
                    edits: &mixed_patch_edits,
                },
            )
            .await?;

        assert_eq!(count_metrics.total, 2);

        assert_eq!(count_metrics.translated, 0);

        assert_eq!(count_metrics.proofread, 0);

        accept(())
    })
    .await
    .unwrap();
}

// Verify a stale identifier rolls back every earlier edit in the batch.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn verify_stale_edit_rollback(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
    (first_id, second_id): (&str, &str),
) {
    let creator_id = page_fixture.chapter_entry.creator_id.clone().into_owned();

    let missing_id = format!("{}missing", PREFIX);

    let stale_orders = [
        UnitOrder {
            id: first_id.to_owned(),
            next_id: Some(second_id.to_owned()),
            is_hidden: false,
        },
        UnitOrder {
            id: second_id.to_owned(),
            next_id: Some(missing_id.clone()),
            is_hidden: false,
        },
        UnitOrder {
            id: missing_id.clone(),
            next_id: None,
            is_hidden: false,
        },
    ];

    let rollback_edits = [
        UnitEdit::Save {
            id: first_id.to_owned(),
            next_id: Patch::Skip,
            is_bubble: None,
            is_flagged: None,
            coord: None,
            translation: Patch::Assign {
                value: UnitTranslation {
                    translated_text: "must roll back".to_string(),
                    last_translator_id: creator_id.clone(),
                },
            },
            revision: Patch::Skip,
        },
        UnitEdit::Save {
            id: missing_id,
            next_id: Patch::Skip,
            is_bubble: None,
            is_flagged: None,
            coord: None,
            translation: Patch::Skip,
            revision: Patch::Skip,
        },
    ];

    let rollback_result = nucl
        .coord(async |context| {
            repo.step(
                context,
                &ApplyUnitEdits {
                    page_id: &page_fixture.page_entry.id,
                    orders: &stale_orders,
                    edits: &rollback_edits,
                },
            )
            .await?;

            accept(())
        })
        .await;

    assert!(matches!(
        rollback_result,
        Err(poprako_orchestra::nucl::Error::Step(
            BaseError::Unrecoverable { .. }
        )),
    ));

    nucl.coord(async |context| {
        //
        let ids = [first_id];

        let selected = repo
            .step(context, &ListUnitInfosByIds { ids: &ids })
            .await?;

        assert_eq!(
            selected
                .as_slice()
                .first()
                .and_then(|unit| unit.translated_text.as_deref()),
            Some("saved after create"),
        );

        accept(())
    })
    .await
    .unwrap();
}

// Create linked units and verify a save in the same batch contributes to the counters.
async fn verify_create_and_save(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
    (first_id, second_id): (&str, &str),
) -> [UnitOrder; 2] {
    let creator_id = page_fixture.chapter_entry.creator_id.clone().into_owned();

    let create_edits = [
        create_edit(first_id, &creator_id, "translated"),
        create_edit(second_id, &creator_id, "second"),
        UnitEdit::Save {
            id: first_id.to_owned(),
            next_id: Patch::Assign {
                value: second_id.to_owned(),
            },
            is_bubble: None,
            is_flagged: None,
            coord: None,
            translation: Patch::Assign {
                value: UnitTranslation {
                    translated_text: "saved after create".to_string(),
                    last_translator_id: creator_id.clone(),
                },
            },
            revision: Patch::Skip,
        },
    ];

    let create_orders = [
        UnitOrder {
            id: first_id.to_owned(),
            next_id: Some(second_id.to_owned()),
            is_hidden: false,
        },
        UnitOrder {
            id: second_id.to_owned(),
            next_id: None,
            is_hidden: false,
        },
    ];

    nucl.coord(async |context| {
        //
        let count_metrics = repo
            .step(
                context,
                &ApplyUnitEdits {
                    page_id: &page_fixture.page_entry.id,
                    orders: &[],
                    edits: &create_edits,
                },
            )
            .await?;

        assert_eq!(count_metrics.total, 2);

        accept(())
    })
    .await
    .unwrap();
    create_orders
}

/// Verifies Unit v2 persistence through the real transaction adapter.
/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub async fn unit_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let page_fixture = test_shared::seed_page(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let first_id = format!("{}first", PREFIX);

    let second_id = format!("{}second", PREFIX);

    let create_orders = verify_create_and_save(
        &repo,
        &nucl,
        &page_fixture,
        (&first_id, &second_id),
    )
    .await;

    verify_stale_edit_rollback(
        &repo,
        &nucl,
        &page_fixture,
        (&first_id, &second_id),
    )
    .await;

    verify_mixed_patches(&repo, &nucl, &page_fixture, (&first_id, &second_id))
        .await;

    verify_tombstone(&repo, &nucl, &page_fixture, &first_id, &create_orders)
        .await;

    verify_restore(&repo, &nucl, &page_fixture, (&first_id, &second_id)).await;

    verify_selected_units(&repo, &nucl, &page_fixture, (&first_id, &second_id))
        .await;

    regression::verify_chunking_and_diff(&repo, &nucl, &page_fixture).await;

    flagged::verify_flags(shared.clone()).await;

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
        is_flagged: false,
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
