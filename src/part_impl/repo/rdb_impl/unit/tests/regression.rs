//! Large-history and counter-semantics coverage for Unit persistence.

use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use crate::model::shared::unit::{UnitCoord, UnitRevision, UnitTranslation};
use crate::model::write::page::PageManifestEntry;
use crate::model::write::unit::UnitEdit;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page::{
    ApplyPageManifest, ListEdittedDiffPageIds,
};
use crate::part::repo::oper::unit::{ApplyUnitEdits, ListUnitOrders};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared::PageFixture;
use crate::result::accept;

use super::{PREFIX, create_edit};

pub(super) async fn verify_chunking_and_diff(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
) {
    let equal_page_id = format!("{}equal-page", PREFIX);

    let missing_translation_page_id =
        format!("{}missing-translation-page", PREFIX);

    let chunked_order_page_id = format!("{}chunked-order-page", PREFIX);

    let additional_pages = [
        PageManifestEntry {
            id: equal_page_id.clone(),
            chapter_id: page_fixture.chapter_entry.id.clone(),
            index: 1,
        },
        PageManifestEntry {
            id: missing_translation_page_id.clone(),
            chapter_id: page_fixture.chapter_entry.id.clone(),
            index: 2,
        },
        PageManifestEntry {
            id: chunked_order_page_id.clone(),
            chapter_id: page_fixture.chapter_entry.id.clone(),
            index: 3,
        },
    ];

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
                        page_id: &chunked_order_page_id,
                    },
                )
                .await?;

            repo.step(
                context,
                &ApplyUnitEdits {
                    page_id: &chunked_order_page_id,
                    orders: &orders,
                    edits: &batch_edits,
                },
            )
            .await?;

            let orders = repo
                .step(
                    context,
                    &ListUnitOrders {
                        page_id: &chunked_order_page_id,
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
                    page_id: &chunked_order_page_id,
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

    nucl.coord(async |context| {
        let orders = repo
            .step(
                context,
                &ListUnitOrders {
                    page_id: &chunked_order_page_id,
                },
            )
            .await?;

        assert_eq!(orders.len(), 600);

        let expected_ids = (0..6)
            .flat_map(|batch_index| {
                (0..100).map(move |unit_index| {
                    format!("{}chunked-{}-{}", PREFIX, batch_index, unit_index,)
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
                page_id: &equal_page_id,
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
                    page_id: &equal_page_id,
                },
            )
            .await?;

        let delete_hidden_diff = [UnitEdit::Delete {
            id: hidden_diff_id.clone(),
        }];

        repo.step(
            context,
            &ApplyUnitEdits {
                page_id: &equal_page_id,
                orders: &orders,
                edits: &delete_hidden_diff,
            },
        )
        .await?;

        accept(())
    })
    .await
    .unwrap();

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
                page_id: &missing_translation_page_id,
                orders: &[],
                edits: &missing_translation_edit,
            },
        )
        .await?;

        accept(())
    })
    .await
    .unwrap();

    let diff_page_ids = repo
        .run(&ListEdittedDiffPageIds {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .unwrap();

    assert_eq!(
        diff_page_ids,
        [
            page_fixture.page_entry.id.clone(),
            missing_translation_page_id,
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
