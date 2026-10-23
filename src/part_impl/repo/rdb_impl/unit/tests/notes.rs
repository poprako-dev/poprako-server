// verify_notes(UnitRepo)(positive): sparse typed annotation writes preserve independent fields and counters.
// verify_notes(UnitRepo)(negative): a failed batch rolls annotations back atomically.

use super::*;

use crate::config::cache::CacheConfig;
use crate::config::repo::RepoConfig;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page::ListPageUnitDiffStats;

// Construct one annotation-only sparse patch.
fn patch(
    id: &str,
    typography_note: Patch<String>,
    translation_note: Patch<String>,
) -> UnitEdit {
    UnitEdit::Save {
        id: id.into(),
        next_id: Patch::Skip,
        is_bubble: None,
        is_flagged: None,
        coord: None,
        typography_note,
        translation_note,
        translation: Patch::Skip,
        revision: Patch::Skip,
    }
}

const FIRST_ID: &str = "rdb-test-unit-domain-notes-a";

const SECOND_ID: &str = "rdb-test-unit-domain-notes-b";

// Exercise create notes through the typed repository.
async fn create_notes(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    fixture: &PageFixture,
) {
    let edits = [
        UnitEdit::Create {
            id: FIRST_ID.into(),
            next_id: None,
            is_bubble: true,
            is_flagged: false,
            coord: UnitCoord {
                x_coord: 0.1,
                y_coord: 0.2,
            },
            typography_note: Some("小字".into()),
            translation_note: Some("存疑".into()),
            translation: None,
            revision: None,
        },
        UnitEdit::Create {
            id: SECOND_ID.into(),
            next_id: None,
            is_bubble: false,
            is_flagged: false,
            coord: UnitCoord {
                x_coord: 0.2,
                y_coord: 0.3,
            },
            typography_note: None,
            translation_note: Some("内部沟通".into()),
            translation: None,
            revision: None,
        },
    ];

    let counts = nucl
        .coord(async |context| {
            repo.step(
                context,
                &ApplyUnitEdits {
                    page_id: &fixture.page_entry.id,
                    orders: &[],
                    edits: &edits,
                },
            )
            .await
        })
        .await
        .unwrap();

    assert_eq!(counts.total, 2);

    assert_eq!(counts.translated, 1);

    assert_eq!(counts.proofread, 0);
}

// Exercise verify patches through the typed repository.
async fn verify_patches(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    fixture: &PageFixture,
) {
    for (edit, expected_note, expected_count) in [
        (patch(FIRST_ID, Patch::Skip, Patch::Clear), Some("小字"), 1),
        (
            patch(
                FIRST_ID,
                Patch::Assign {
                    value: "小字\n\n括号".into(),
                },
                Patch::Skip,
            ),
            Some("小字\n\n括号"),
            1,
        ),
        (
            UnitEdit::Delete {
                id: FIRST_ID.into(),
            },
            Some("小字\n\n括号"),
            0,
        ),
        (
            patch(FIRST_ID, Patch::Skip, Patch::Skip),
            Some("小字\n\n括号"),
            1,
        ),
        (patch(FIRST_ID, Patch::Clear, Patch::Skip), None, 0),
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

            let counts = repo
                .step(
                    context,
                    &ApplyUnitEdits {
                        page_id: &fixture.page_entry.id,
                        orders: &orders,
                        edits: &[edit],
                    },
                )
                .await?;

            assert_eq!(counts.translated, expected_count);

            let units = repo
                .step(
                    context,
                    &ListUnitInfosByIds {
                        ids: &[FIRST_ID, SECOND_ID],
                    },
                )
                .await?;

            let first = units.iter().find(|unit| unit.id == FIRST_ID).unwrap();

            let second =
                units.iter().find(|unit| unit.id == SECOND_ID).unwrap();

            assert_eq!(first.typography_note.as_deref(), expected_note);

            assert!(first.translation_note.is_none());

            assert!(first.translated_text.is_none());

            assert!(first.last_translator_id.is_none());

            assert!(first.last_proofreader_id.is_none());

            assert_eq!(second.translation_note.as_deref(), Some("内部沟通"));

            accept(())
        })
        .await
        .unwrap();
    }
}

// Exercise verify diff stats through the typed repository.
async fn verify_diff_stats(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    fixture: &PageFixture,
) {
    nucl.coord(async |context| {
        let orders = repo
            .step(
                context,
                &ListUnitOrders {
                    page_id: &fixture.page_entry.id,
                },
            )
            .await?;

        let revision = UnitEdit::Save {
            id: SECOND_ID.into(),
            next_id: Patch::Skip,
            is_bubble: None,
            is_flagged: None,
            coord: None,
            typography_note: Patch::Skip,
            translation_note: Patch::Skip,
            translation: Patch::Skip,
            revision: Patch::Assign {
                value: UnitRevision {
                    is_proofread: false,
                    proofread_text: Some("appended body".into()),
                    last_proofreader_id: fixture
                        .chapter_info
                        .creator_id
                        .clone(),
                },
            },
        };

        let counts = repo
            .step(
                context,
                &ApplyUnitEdits {
                    page_id: &fixture.page_entry.id,
                    orders: &orders,
                    edits: &[
                        patch(
                            FIRST_ID,
                            Patch::Assign {
                                value: "小字".into(),
                            },
                            Patch::Skip,
                        ),
                        revision,
                    ],
                },
            )
            .await?;

        assert_eq!(counts.translated, 2);

        accept(())
    })
    .await
    .unwrap();

    let stats = repo
        .run(&ListPageUnitDiffStats {
            chapter_id: &fixture.chapter_info.id,
        })
        .await
        .unwrap();

    assert_eq!(stats.first().unwrap().translated_unit_count, 1);

    assert_eq!(stats.first().unwrap().editted_unit_count, 0);

    assert_eq!(stats.first().unwrap().proofreader_append_unit_count, 1);
}

// Exercise verify rollback through the typed repository.
async fn verify_rollback(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    fixture: &PageFixture,
) {
    let failed = nucl
        .coord(async |context| {
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
                    edits: &[patch(
                        FIRST_ID,
                        Patch::Assign {
                            value: "rolled back".into(),
                        },
                        Patch::Skip,
                    )],
                },
            )
            .await?;

            repo.step(
                context,
                &ApplyUnitEdits {
                    page_id: &fixture.page_entry.id,
                    orders: &orders,
                    edits: &[patch("missing", Patch::Clear, Patch::Skip)],
                },
            )
            .await
        })
        .await;

    assert!(failed.is_err());

    let units = repo
        .run(&ListUnitInfosByPageIds {
            page_ids: &[&fixture.page_entry.id],
        })
        .await
        .unwrap();

    assert_eq!(
        units
            .iter()
            .find(|unit| unit.id == FIRST_ID)
            .unwrap()
            .typography_note
            .as_deref(),
        Some("小字")
    );
}

pub async fn verify_notes(shared: RdbCore) {
    let fixture =
        test_shared::seed_page(&shared, "rdb-test-unit-domain-notes-").await;

    let repo = HybRepo::new(
        shared.clone(),
        CacheConfig::default(),
        RepoConfig::default(),
    );

    let nucl = RdbNucl::<ReptRead>::new(shared);

    create_notes(&repo, &nucl, &fixture).await;

    verify_patches(&repo, &nucl, &fixture).await;

    verify_diff_stats(&repo, &nucl, &fixture).await;

    verify_rollback(&repo, &nucl, &fixture).await;
}
