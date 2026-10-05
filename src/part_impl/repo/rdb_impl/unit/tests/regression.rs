//! Large-history and counter-semantics coverage for Unit persistence.

use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::model::write::page::PageManifestEntry;
use crate::model::write::unit::UnitEdit;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page::{ApplyPageManifest, ListPageUnitDiffStats};
use crate::part::repo::oper::unit::{
    ApplyUnitEdits, ListUnitInfosByPageIds, ListUnitOrders,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared::PageFixture;
use crate::result::accept;

use super::{PREFIX, create_edit};

// Verify repeated page identifiers preserve history ordering without duplicate rows.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn verify_selected_history(
    repo: &HybRepo,
    missing_translation_page_id: &str,
    chunked_order_page_id: &str,
    equal_page_id: &str,
    hidden_diff_id: &str,
) {
    let selected_infos = repo
        .run(&ListUnitInfosByPageIds {
            page_ids: &[
                missing_translation_page_id,
                chunked_order_page_id,
                equal_page_id,
                chunked_order_page_id,
            ],
        })
        .await
        .unwrap();

    assert_eq!(selected_infos.len(), 607);

    assert_eq!(
        selected_infos.as_slice().first().unwrap().id,
        format!("{}missing-translation", PREFIX)
    );

    assert_eq!(
        selected_infos.get(1).unwrap().id,
        format!("{}chunked-0-0", PREFIX)
    );

    assert_eq!(
        selected_infos.get(600).unwrap().id,
        format!("{}chunked-5-99", PREFIX)
    );

    assert!(
        selected_infos
            .get(1..601)
            .unwrap()
            .iter()
            .all(|unit_info| unit_info.hidden_at.is_some())
    );

    assert_eq!(
        selected_infos.get(601).unwrap().id,
        format!("{}equal", PREFIX)
    );

    assert_eq!(selected_infos.get(606).unwrap().id, hidden_diff_id);
}

// Seed visible and hidden copies of every translation and revision pairing.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn seed_text_matrix(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
    matrix_page_id: &str,
) {
    let text_cases = [
        (Some("translated"), None),
        (Some("same"), Some("same")),
        (Some("translated"), Some("revised")),
        (Some("A"), Some("a")),
        (Some("text"), Some(" text ")),
        (Some("translated"), Some("")),
        (Some("translated"), Some(" \t\u{3000}")),
        (None, Some("appended")),
        (Some(""), Some("appended")),
        (Some(" \t\r\n"), Some("appended")),
        (Some("\u{0085}\u{00a0}\u{3000}"), Some("appended")),
        (None, None),
        (Some(" "), Some("\u{3000}")),
    ];

    let mut matrix_edits = Vec::new();

    let mut hidden_edits = Vec::new();

    for (index, (translation, revision)) in text_cases.into_iter().enumerate() {
        matrix_edits.push(create_text_edit(
            &format!("{}matrix-{index}", PREFIX),
            &page_fixture.chapter_entry.creator_id,
            translation,
            revision,
            index % 2 == 0,
        ));

        let hidden_id = format!("{}matrix-hidden-{index}", PREFIX);

        matrix_edits.push(create_text_edit(
            &hidden_id,
            &page_fixture.chapter_entry.creator_id,
            translation,
            revision,
            index % 2 != 0,
        ));

        hidden_edits.push(UnitEdit::Delete { id: hidden_id });
    }

    nucl.coord(async |context| {
        repo.step(
            context,
            &ApplyUnitEdits {
                page_id: matrix_page_id,
                orders: &[],
                edits: &matrix_edits,
            },
        )
        .await?;

        let orders = repo
            .step(
                context,
                &ListUnitOrders {
                    page_id: matrix_page_id,
                },
            )
            .await?;

        repo.step(
            context,
            &ApplyUnitEdits {
                page_id: matrix_page_id,
                orders: &orders,
                edits: &hidden_edits,
            },
        )
        .await?;

        accept(())
    })
    .await
    .unwrap();
}

// Seed a proofreader append without a source translation.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn seed_missing_translation(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
    missing_translation_page_id: &str,
) {
    let missing_translation_edit = [create_text_edit(
        &format!("{}missing-translation", PREFIX),
        &page_fixture.chapter_entry.creator_id,
        None,
        Some("proofread"),
        false,
    )];

    nucl.coord(async |context| {
        repo.step(
            context,
            &ApplyUnitEdits {
                page_id: missing_translation_page_id,
                orders: &[],
                edits: &missing_translation_edit,
            },
        )
        .await?;

        accept(())
    })
    .await
    .unwrap();
}

// Seed equal, empty, whitespace, missing-proofread, and hidden translations.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn seed_excluded_diff_cases(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
    equal_page_id: &str,
) -> String {
    let hidden_diff_id = format!("{}hidden-diff", PREFIX);

    let excluded_edits = [
        create_text_edit(
            &format!("{}equal", PREFIX),
            &page_fixture.chapter_entry.creator_id,
            Some("same"),
            Some("same"),
            true,
        ),
        create_text_edit(
            &format!("{}empty", PREFIX),
            &page_fixture.chapter_entry.creator_id,
            None,
            Some(""),
            true,
        ),
        create_text_edit(
            &format!("{}ascii-whitespace", PREFIX),
            &page_fixture.chapter_entry.creator_id,
            None,
            Some(" \t\r\n"),
            true,
        ),
        create_text_edit(
            &format!("{}unicode-whitespace", PREFIX),
            &page_fixture.chapter_entry.creator_id,
            None,
            Some("\u{3000}"),
            true,
        ),
        create_text_edit(
            &format!("{}missing-proofread", PREFIX),
            &page_fixture.chapter_entry.creator_id,
            Some("translated"),
            None,
            true,
        ),
        create_text_edit(
            &hidden_diff_id,
            &page_fixture.chapter_entry.creator_id,
            Some("translated"),
            Some("hidden proofread"),
            true,
        ),
    ];

    nucl.coord(async |context| {
        repo.step(
            context,
            &ApplyUnitEdits {
                page_id: equal_page_id,
                orders: &[],
                edits: &excluded_edits,
            },
        )
        .await?;

        accept(())
    })
    .await
    .unwrap();

    nucl.coord(async |context| {
        let orders = repo
            .step(
                context,
                &ListUnitOrders {
                    page_id: equal_page_id,
                },
            )
            .await?;

        let delete_hidden_diff = [UnitEdit::Delete {
            id: hidden_diff_id.clone(),
        }];

        repo.step(
            context,
            &ApplyUnitEdits {
                page_id: equal_page_id,
                orders: &orders,
                edits: &delete_hidden_diff,
            },
        )
        .await?;

        accept(())
    })
    .await
    .unwrap();
    hidden_diff_id
}

// Verify all historical identifiers retain insertion order and hidden flags.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn verify_chunked_order(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    chunked_order_page_id: &str,
) {
    nucl.coord(async |context| {
        let orders = repo
            .step(
                context,
                &ListUnitOrders {
                    page_id: chunked_order_page_id,
                },
            )
            .await?;

        assert_eq!(orders.len(), 600);

        let expected_ids = (0..6)
            .flat_map(|batch_index| {
                (0..100).map(move |unit_index| {
                    format!("{}chunked-{}-{}", PREFIX, batch_index, unit_index)
                })
            })
            .collect::<Vec<_>>();

        let actual_ids = orders
            .iter()
            .map(|order| order.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(actual_ids, expected_ids);

        assert!(orders.iter().all(|order| order.is_hidden));

        accept(())
    })
    .await
    .unwrap();
}

// Create and tombstone six batches so history exceeds one database chunk.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn seed_chunked_history(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
    chunked_order_page_id: &str,
) {
    for batch_index in 0..6 {
        let batch_edits = (0..100)
            .map(|unit_index| {
                let unit_id = format!(
                    "{}chunked-{}-{}",
                    PREFIX, batch_index, unit_index,
                );

                create_edit(
                    &unit_id,
                    &page_fixture.chapter_entry.creator_id,
                    "chunked",
                )
            })
            .collect::<Vec<_>>();

        nucl.coord(async |context| {
            let orders = repo
                .step(
                    context,
                    &ListUnitOrders {
                        page_id: chunked_order_page_id,
                    },
                )
                .await?;

            repo.step(
                context,
                &ApplyUnitEdits {
                    page_id: chunked_order_page_id,
                    orders: &orders,
                    edits: &batch_edits,
                },
            )
            .await?;

            let orders = repo
                .step(
                    context,
                    &ListUnitOrders {
                        page_id: chunked_order_page_id,
                    },
                )
                .await?;

            let delete_edits = batch_edits
                .iter()
                .filter_map(|edit| match edit {
                    UnitEdit::Create { id, .. } => {
                        Some(UnitEdit::Delete { id: id.clone() })
                    }
                    UnitEdit::Save { .. } | UnitEdit::Delete { .. } => None,
                })
                .collect::<Vec<_>>();

            repo.step(
                context,
                &ApplyUnitEdits {
                    page_id: chunked_order_page_id,
                    orders: &orders,
                    edits: &delete_edits,
                },
            )
            .await?;

            accept(())
        })
        .await
        .unwrap();
    }
}

// Create dedicated pages for equality, append-only, hidden history, and text matrix cases.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn seed_diff_pages(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
) -> [String; 4] {
    let equal_page_id = format!("{}equal-page", PREFIX);

    let missing_translation_page_id =
        format!("{}missing-translation-page", PREFIX);

    let chunked_order_page_id = format!("{}chunked-order-page", PREFIX);

    let matrix_page_id = format!("{}matrix-page", PREFIX);

    let empty_page_id = format!("{}empty-page", PREFIX);

    let additional_pages = [
        (matrix_page_id.as_str(), 4),
        (empty_page_id.as_str(), 5),
        (equal_page_id.as_str(), 1),
        (missing_translation_page_id.as_str(), 2),
        (chunked_order_page_id.as_str(), 3),
    ]
    .into_iter()
    .map(|(id, index)| PageManifestEntry {
        id: id.to_owned(),
        chapter_id: page_fixture.chapter_entry.id.clone(),
        index,
    })
    .collect::<Vec<_>>();

    nucl.coord(async |context| {
        repo.step(
            context,
            &ApplyPageManifest {
                entries: &additional_pages,
            },
        )
        .await?;

        accept(())
    })
    .await
    .unwrap();
    [
        equal_page_id,
        missing_translation_page_id,
        chunked_order_page_id,
        matrix_page_id,
    ]
}

#[tokio::test]
#[serial_test::serial(repo_rdb)]
async fn unit_diff_stats_use_testcontainer() {
    let test_rdb = crate::shared::test_rdb::start().await;

    super::unit_roundtrip_uses_testcontainer(test_rdb.core()).await;
}

pub(super) async fn verify_chunking_and_diff(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
) {
    let [
        equal_page_id,
        missing_translation_page_id,
        chunked_order_page_id,
        matrix_page_id,
    ] = seed_diff_pages(repo, nucl, page_fixture).await;

    seed_chunked_history(repo, nucl, page_fixture, &chunked_order_page_id)
        .await;

    verify_chunked_order(repo, nucl, &chunked_order_page_id).await;

    let hidden_diff_id =
        seed_excluded_diff_cases(repo, nucl, page_fixture, &equal_page_id)
            .await;

    seed_missing_translation(
        repo,
        nucl,
        page_fixture,
        &missing_translation_page_id,
    )
    .await;

    seed_text_matrix(repo, nucl, page_fixture, &matrix_page_id).await;

    verify_selected_history(
        repo,
        &missing_translation_page_id,
        &chunked_order_page_id,
        &equal_page_id,
        &hidden_diff_id,
    )
    .await;

    let page_unit_diff_stats = repo
        .run(&ListPageUnitDiffStats {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .unwrap();

    assert_eq!(
        page_unit_diff_stats
            .iter()
            .map(|stats| (
                stats.page_id.as_str(),
                stats.index,
                stats.translated_unit_count,
                stats.editted_unit_count,
                stats.proofreader_append_unit_count,
            ))
            .collect::<Vec<_>>(),
        [
            (page_fixture.page_entry.id.as_str(), 0, 0, 0, 1),
            (missing_translation_page_id.as_str(), 2, 0, 0, 1),
            (matrix_page_id.as_str(), 4, 7, 3, 4),
        ]
    );
}

fn create_text_edit(
    id: &str,
    user_id: &str,
    translated_text: Option<&str>,
    proofread_text: Option<&str>,
    is_proofread: bool,
) -> UnitEdit {
    UnitEdit::Create {
        id: id.to_string(),
        next_id: None,
        is_bubble: true,
        is_flagged: false,
        coord: UnitCoord {
            x_coord: 1.0,
            y_coord: 2.0,
        },
        translation: translated_text.map(|translated_text| UnitTranslation {
            translated_text: translated_text.to_string(),
            last_translator_id: user_id.to_string(),
        }),
        revision: Some(UnitRevision {
            is_proofread,
            proofread_text: proofread_text.map(str::to_string),
            last_proofreader_id: user_id.to_string(),
        }),
    }
}
