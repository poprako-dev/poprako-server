// verify_flags(UnitRepo)(positive): typed batch writes persist flags and statistics exclude tombstones.

use super::*;

use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page::ListPageUnitFlaggedStats;

pub async fn verify_flags(shared: RdbCore) {
    let fixture =
        test_shared::seed_page(&shared, "rdb-test-unit-domain-flagged-").await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared);

    let mut first = create_edit(
        "rdb-test-unit-domain-flagged-a",
        &fixture.chapter_entry.creator_id,
        "flagged",
    );

    let UnitEdit::Create { is_flagged, .. } = &mut first else {
        panic!("expected create");
    };

    *is_flagged = true;

    let second = create_edit(
        "rdb-test-unit-domain-flagged-b",
        &fixture.chapter_entry.creator_id,
        "unflagged",
    );

    nucl.coord(async |context| {
        repo.step(
            context,
            &ApplyUnitEdits {
                page_id: &fixture.page_entry.id,
                orders: &[],
                edits: &[first, second],
            },
        )
        .await?;

        accept(())
    })
    .await
    .unwrap();

    let stats = repo
        .run(&ListPageUnitFlaggedStats {
            chapter_id: &fixture.chapter_entry.id,
        })
        .await
        .unwrap();

    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].page_id, fixture.page_entry.id);
    assert_eq!(stats[0].index, fixture.page_entry.index);
    assert_eq!(stats[0].flagged_unit_count, 1);

    let first_id = "rdb-test-unit-domain-flagged-a";

    for (edit, expected_count, expected_flag) in [
        (
            UnitEdit::Delete {
                id: first_id.into(),
            },
            0,
            true,
        ),
        (patch(first_id, None), 1, true),
        (patch(first_id, Some(false)), 0, false),
        (patch(first_id, Some(true)), 1, true),
    ] {
        nucl.coord(async |context| {
            let orders = repo
                .step(
                    context,
                    &ListUnitOrders {
                        page_id: &fixture.page_entry.id,
                    },
                )
                .await?;

            repo.step(
                context,
                &ApplyUnitEdits {
                    page_id: &fixture.page_entry.id,
                    orders: &orders,
                    edits: &[edit],
                },
            )
            .await?;

            let units = repo
                .step(context, &ListUnitInfosByIds { ids: &[first_id] })
                .await?;

            assert_eq!(units[0].is_flagged, expected_flag);
            assert_eq!(units[0].translated_text.as_deref(), Some("flagged"));
            assert_eq!(
                units[0].last_translator_id.as_deref(),
                Some(fixture.chapter_entry.creator_id.as_ref())
            );

            accept(())
        })
        .await
        .unwrap();

        let stats = repo
            .run(&ListPageUnitFlaggedStats {
                chapter_id: &fixture.chapter_entry.id,
            })
            .await
            .unwrap();

        assert_eq!(
            stats
                .iter()
                .map(|stat| stat.flagged_unit_count)
                .sum::<usize>(),
            expected_count
        );
        assert_eq!(stats.len(), expected_count);
    }

    let units = repo
        .run(&ListUnitInfosByPageIds {
            page_ids: &[fixture.page_entry.id.as_str()],
        })
        .await
        .unwrap();

    assert!(units[0].is_flagged);
    assert!(!units[1].is_flagged);
}

// Build a content-preserving flag patch.
fn patch(id: &str, is_flagged: Option<bool>) -> UnitEdit {
    UnitEdit::Save {
        id: id.into(),
        next_id: Patch::Skip,
        is_bubble: None,
        is_flagged,
        coord: None,
        translation: Patch::Skip,
        revision: Patch::Skip,
    }
}
