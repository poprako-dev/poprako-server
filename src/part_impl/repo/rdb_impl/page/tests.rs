#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// page_roundtrip_uses_testcontainer(SetPageUnitCountMetrics, ListPageInfos)(positive): page repo persists and updates page counts in an isolated PostgreSQL container.

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use poprako_rdb_core::RdbCore;

use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitCountMetrics;
use crate::model::write::page::PageManifestEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page::{
    ApplyPageManifest, GetPageInfo, ListFirstPageInfos, ListPageInfos,
    ListPageInfosExcluded, ListPageUnitDiffStats, SetPageUnitCountMetrics,
    ShiftPageIndexesTemporary,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::schema::t_chapter;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::part_impl::repo::rdb_impl::test_shared::PageFixture;
use crate::result::{BaseError, ExpectedVariant};
use crate::value::page::MAX_CHAPTER_PAGE_COUNT;
use time::OffsetDateTime;

const PREFIX: &str = "rdb-test-page-domain-";

// Verify identity, creation time, and all counters together after reordering.
fn assert_page_order(
    page_infos: &[PageInfo],
    expected: [(&str, OffsetDateTime, (usize, usize, usize)); 2],
) {
    //
    assert_eq!(page_infos.len(), expected.len());

    for (page_info, (id, created_at, (total, translated, proofread))) in
        page_infos.iter().zip(expected)
    {
        assert_eq!(page_info.id, id);

        assert_eq!(page_info.created_at, created_at);

        assert_eq!(page_info.total_unit_count, total);

        assert_eq!(page_info.translated_unit_count, translated);

        assert_eq!(page_info.proofread_unit_count, proofread);
    }
}

// Set and read all page counters before changing the manifest.
async fn verify_page_counters(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
) -> OffsetDateTime {
    let unit_count_metrics = UnitCountMetrics {
        total: 2,
        translated: 1,
        proofread: 1,
    };

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &SetPageUnitCountMetrics {
                id: &page_fixture.page_entry.id,
                count_metrics: unit_count_metrics,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let page_infos = repo
        .run(&ListPageInfos {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .unwrap();

    assert_eq!(page_infos.len(), 1);

    let page_info = page_infos.as_slice().first().unwrap();

    assert_eq!(page_info.total_unit_count, 2);

    assert_eq!(page_info.translated_unit_count, 1);

    assert_eq!(page_info.proofread_unit_count, 1);

    page_info.created_at
}

// Verify a newly created page starts with empty counters and a current timestamp.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn create_second_page(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
) -> (PageManifestEntry, OffsetDateTime) {
    let second_page_entry = PageManifestEntry {
        id: format!("{}page-later", PREFIX),
        chapter_id: page_fixture.chapter_entry.id.clone(),
        index: 1,
    };

    let new_page_info = nucl
        .coord(async |context| {
            //
            let page_infos = repo
                .step(
                    context,
                    &ApplyPageManifest {
                        entries: std::slice::from_ref(&second_page_entry),
                    },
                )
                .await?;

            page_infos.into_iter().next().ok_or_else(|| {
                BaseError::Unrecoverable {
                    msg: "page creation returned no row".into(),
                }
            })
        })
        .await
        .unwrap();

    assert_eq!(new_page_info.total_unit_count, 0);

    assert_eq!(new_page_info.translated_unit_count, 0);

    assert_eq!(new_page_info.proofread_unit_count, 0);

    let new_created_at = new_page_info.created_at;

    assert!(new_created_at <= time::OffsetDateTime::now_utc());
    (second_page_entry, new_created_at)
}

// Swap page positions and verify the first-page query and retained metadata.
async fn verify_manifest_reorder(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
    second_page_entry: &PageManifestEntry,
    (retained_created_at, new_created_at): (OffsetDateTime, OffsetDateTime),
) {
    let manifest_entries = vec![
        PageManifestEntry {
            id: second_page_entry.id.clone(),
            chapter_id: page_fixture.chapter_entry.id.clone(),
            index: 0,
        },
        PageManifestEntry {
            id: page_fixture.page_entry.id.clone(),
            chapter_id: page_fixture.chapter_entry.id.clone(),
            index: 1,
        },
    ];

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &ShiftPageIndexesTemporary {
                chapter_id: &page_fixture.chapter_entry.id,
            },
        )
        .await?;

        let page_infos = repo
            .step(
                context,
                &ApplyPageManifest {
                    entries: &manifest_entries,
                },
            )
            .await?;

        assert_eq!(page_infos.len(), 2);

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let reordered_page_infos = repo
        .run(&ListPageInfos {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .unwrap();

    assert_page_order(
        &reordered_page_infos,
        [
            (&second_page_entry.id, new_created_at, (0, 0, 0)),
            (&page_fixture.page_entry.id, retained_created_at, (2, 1, 1)),
        ],
    );

    let chapter_ids = vec![page_fixture.chapter_entry.id.as_str()];

    let first_page_infos = repo
        .run(&ListFirstPageInfos {
            chapter_ids: &chapter_ids,
        })
        .await
        .unwrap();

    let first_page_info = first_page_infos
        .iter()
        .find(|page_info| page_info.chapter_id == page_fixture.chapter_entry.id)
        .expect("first page info for the chapter");

    assert_eq!(first_page_info.id, second_page_entry.id);
}

// Force a manifest transaction failure and verify all original metadata survives.
async fn verify_manifest_rollback(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
    second_page_entry: &PageManifestEntry,
    (retained_created_at, new_created_at): (OffsetDateTime, OffsetDateTime),
) {
    let rollback_entries = vec![
        PageManifestEntry {
            id: page_fixture.page_entry.id.clone(),
            chapter_id: page_fixture.chapter_entry.id.clone(),
            index: 0,
        },
        PageManifestEntry {
            id: second_page_entry.id.clone(),
            chapter_id: page_fixture.chapter_entry.id.clone(),
            index: 1,
        },
    ];

    let rollback_result = nucl
        .coord(async |context| {
            //
            repo.step(
                context,
                &ShiftPageIndexesTemporary {
                    chapter_id: &page_fixture.chapter_entry.id,
                },
            )
            .await?;

            repo.step(
                context,
                &ApplyPageManifest {
                    entries: &rollback_entries,
                },
            )
            .await?;

            Err::<(), BaseError>(BaseError::Unrecoverable {
                msg: "force page-manifest rollback".into(),
            })
        })
        .await;

    assert!(rollback_result.is_err());

    let rolled_back_page_infos = repo
        .run(&ListPageInfos {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .unwrap();

    assert_page_order(
        &rolled_back_page_infos,
        [
            (&second_page_entry.id, new_created_at, (0, 0, 0)),
            (&page_fixture.page_entry.id, retained_created_at, (2, 1, 1)),
        ],
    );
}

// Verify page lookup hides the tombstoned parent chapter.
async fn verify_tombstoned_page(
    shared: &RdbCore,
    repo: &HybRepo,
    page_fixture: &PageFixture,
) {
    let mut conn = shared.get().await.unwrap();

    diesel::update(
        t_chapter::table
            .filter(t_chapter::f_id.eq(&page_fixture.chapter_entry.id)),
    )
    .set(t_chapter::f_deleted_at.eq(Some(time::OffsetDateTime::now_utc())))
    .execute(&mut conn)
    .await
    .unwrap();

    drop(conn);

    let page_error = repo
        .run(&GetPageInfo {
            id: &page_fixture.page_entry.id,
        })
        .await
        .err()
        .unwrap();

    assert!(matches!(
        page_error,
        BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        }
    ));
}

// Verify every bounded page list rejects an overflowing stored manifest.
async fn verify_page_count_bound(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    page_fixture: &PageFixture,
) {
    let overflow_entries = (2..=MAX_CHAPTER_PAGE_COUNT)
        .map(|index| PageManifestEntry {
            id: format!("{PREFIX}page-overflow-{index:03}"),
            chapter_id: page_fixture.chapter_entry.id.clone(),
            index,
        })
        .collect::<Vec<_>>();

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &ApplyPageManifest {
                entries: &overflow_entries,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let list_error = repo
        .run(&ListPageInfos {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .err()
        .unwrap();

    assert!(matches!(list_error, BaseError::Unrecoverable { .. }));

    let diff_error = repo
        .run(&ListPageUnitDiffStats {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .err()
        .unwrap();

    assert!(matches!(diff_error, BaseError::Unrecoverable { .. }));

    let excluded_error = nucl
        .coord(async |context| {
            repo.step(
                context,
                &ListPageInfosExcluded {
                    chapter_id: &page_fixture.chapter_entry.id,
                },
            )
            .await
        })
        .await
        .map_err(BaseError::from)
        .err()
        .unwrap();

    assert!(matches!(excluded_error, BaseError::Unrecoverable { .. }));
}

/// Verifies page roundtrip via testcontainers.
/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
pub async fn page_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let page_fixture = test_shared::seed_page(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let retained_created_at =
        verify_page_counters(&repo, &nucl, &page_fixture).await;

    let (second_page_entry, new_created_at) =
        create_second_page(&repo, &nucl, &page_fixture).await;

    verify_manifest_reorder(
        &repo,
        &nucl,
        &page_fixture,
        &second_page_entry,
        (retained_created_at, new_created_at),
    )
    .await;

    verify_manifest_rollback(
        &repo,
        &nucl,
        &page_fixture,
        &second_page_entry,
        (retained_created_at, new_created_at),
    )
    .await;

    verify_tombstoned_page(&shared, &repo, &page_fixture).await;

    verify_page_count_bound(&repo, &nucl, &page_fixture).await;

    test_shared::cleanup(&shared, PREFIX).await.unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .unwrap();
}

/// Verifies original filename upsert, rollback, foreign keys, and page cascades.
/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
pub async fn raw_ident_roundtrip_uses_testcontainer(shared: RdbCore) {
    use crate::model::write::page::PageRawIdentsRepl;
    use crate::part::repo::oper::page::{
        DeletePages, ListPageRawIdentInfos, UpdatePageRawIdents,
    };

    const RAW_PREFIX: &str = "rdb-test-page-raw-ident-";

    test_shared::reset(&shared, RAW_PREFIX).await;

    let fixture = test_shared::seed_page(&shared, RAW_PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let page_id = fixture.page_entry.id.as_str();

    let page_ids = [page_id];

    let mut created_at = None;

    for (repl, expected) in [
        (Some("原稿 01.JPG"), Some("原稿 01.JPG")),
        (Some("原稿 01.JPG"), Some("原稿 01.JPG")),
        (Some("renamed.png"), Some("renamed.png")),
        (None, None),
        (None, None),
    ] {
        let repls = PageRawIdentsRepl {
            idents: &[(page_id, repl)],
        };

        nucl.coord(async |context| {
            repo.step(context, &UpdatePageRawIdents { repl: &repls })
                .await
        })
        .await
        .unwrap();

        let infos = repo
            .run(&ListPageRawIdentInfos {
                page_ids: &page_ids,
            })
            .await
            .unwrap();

        assert_eq!(
            infos.as_slice().first().map(|info| info.raw_ident.as_str()),
            expected
        );

        if let Some(info) = infos.as_slice().first() {
            assert_eq!(
                *created_at.get_or_insert(info.created_at),
                info.created_at
            );
            assert!(info.updated_at >= info.created_at);
        }
    }

    let repls = PageRawIdentsRepl {
        idents: &[
            (page_id, Some("rollback.png")),
            ("missing-page", Some("invalid.png")),
        ],
    };

    assert!(
        nucl.coord(async |context| repo
            .step(context, &UpdatePageRawIdents { repl: &repls })
            .await)
            .await
            .is_err()
    );
    assert!(
        repo.run(&ListPageRawIdentInfos {
            page_ids: &page_ids
        })
        .await
        .unwrap()
        .is_empty()
    );

    let repls = PageRawIdentsRepl {
        idents: &[(page_id, Some("delete.png"))],
    };

    nucl.coord(async |context| {
        repo.step(context, &UpdatePageRawIdents { repl: &repls })
            .await
    })
    .await
    .unwrap();

    let ids = [page_id.to_owned()];

    nucl.coord(async |context| {
        repo.step(context, &DeletePages::Ids { ids: &ids }).await
    })
    .await
    .unwrap();

    assert!(
        repo.run(&ListPageRawIdentInfos {
            page_ids: &page_ids
        })
        .await
        .unwrap()
        .is_empty()
    );
    assert!(
        repo.run(&ListPageRawIdentInfos { page_ids: &[] })
            .await
            .unwrap()
            .is_empty()
    );

    test_shared::cleanup(&shared, RAW_PREFIX).await.unwrap();
}
