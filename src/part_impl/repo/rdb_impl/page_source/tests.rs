#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// page_source_roundtrip_uses_testcontainer(SetPageSourceUnitCountMetrics, ListPageSourceInfos)(positive): page repo persists and updates page counts in an isolated PostgreSQL container.

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use poprako_rdb_core::RdbCore;

use crate::config::AppConfig;
use crate::model::read::proj::page_source::PageSourceInfo;
use crate::model::read::proj::unit::UnitCountMetrics;
use crate::model::write::page_source::PageSourceManifestEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page_source::{
    ApplyPageSourceManifest, GetPageSourceInfo, ListFirstPageSourceInfos,
    ListPageSourceInfos, ListPageSourceInfosExcluded,
    ListPageSourceUnitDiffStats, SetPageSourceUnitCountMetrics,
    ShiftPageSourceIndexesTemporary,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::RdbRepo;
use crate::part_impl::repo::rdb_impl::schema::t_chapter;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::part_impl::repo::rdb_impl::test_shared::PageSourceFixture;
use crate::result::{BaseError, ExpectedVariant};
use crate::value::page_source::MAX_CHAPTER_PAGE_SOURCE_COUNT;
use time::OffsetDateTime;

const PREFIX: &str = "rdb-test-page-domain-";

// Verify identity, creation time, and all counters together after reordering.
fn assert_page_source_order(
    page_source_infos: &[PageSourceInfo],
    expected: [(&str, OffsetDateTime, (usize, usize, usize)); 2],
) {
    //
    assert_eq!(page_source_infos.len(), expected.len());

    for (page_source_info, (id, created_at, (total, translated, proofread))) in
        page_source_infos.iter().zip(expected)
    {
        assert_eq!(page_source_info.id, id);

        assert_eq!(page_source_info.created_at, created_at);

        assert_eq!(page_source_info.total_unit_count, total);

        assert_eq!(page_source_info.translated_unit_count, translated);

        assert_eq!(page_source_info.proofread_unit_count, proofread);
    }
}

// Set and read all page counters before changing the manifest.
async fn verify_page_source_counters(
    repo: &RdbRepo,
    nucl: &RdbNucl<ReptRead>,
    page_source_fixture: &PageSourceFixture,
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
            &SetPageSourceUnitCountMetrics {
                id: &page_source_fixture.page_source_entry.id,
                count_metrics: unit_count_metrics,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let page_source_infos = repo
        .run(&ListPageSourceInfos {
            chapter_id: &page_source_fixture.chapter_info.id,
        })
        .await
        .unwrap();

    assert_eq!(page_source_infos.len(), 1);

    let page_source_info = page_source_infos.as_slice().first().unwrap();

    assert_eq!(page_source_info.total_unit_count, 2);

    assert_eq!(page_source_info.translated_unit_count, 1);

    assert_eq!(page_source_info.proofread_unit_count, 1);

    page_source_info.created_at
}

// Verify a newly created page starts with empty counters and a current timestamp.
async fn create_second_page(
    repo: &RdbRepo,
    nucl: &RdbNucl<ReptRead>,
    page_source_fixture: &PageSourceFixture,
) -> (PageSourceManifestEntry, OffsetDateTime) {
    let second_page_source_entry = PageSourceManifestEntry {
        id: [PREFIX, "page-later"].concat(),
        chapter_id: page_source_fixture.chapter_info.id.clone(),
        index: 1,
    };

    let new_page_source_info = nucl
        .coord(async |context| {
            //
            let page_source_infos = repo
                .step(
                    context,
                    &ApplyPageSourceManifest {
                        entries: std::slice::from_ref(
                            &second_page_source_entry,
                        ),
                    },
                )
                .await?;

            page_source_infos.into_iter().next().ok_or_else(|| {
                BaseError::Unrecoverable {
                    message: "page creation returned no record".into(),
                }
            })
        })
        .await
        .unwrap();

    assert_eq!(new_page_source_info.total_unit_count, 0);

    assert_eq!(new_page_source_info.translated_unit_count, 0);

    assert_eq!(new_page_source_info.proofread_unit_count, 0);

    let new_created_at = new_page_source_info.created_at;

    assert!(new_created_at <= time::OffsetDateTime::now_utc());
    (second_page_source_entry, new_created_at)
}

// Swap page positions and verify the first-page query and retained metadata.
async fn verify_manifest_reorder(
    repo: &RdbRepo,
    nucl: &RdbNucl<ReptRead>,
    page_source_fixture: &PageSourceFixture,
    second_page_source_entry: &PageSourceManifestEntry,
    (retained_created_at, new_created_at): (OffsetDateTime, OffsetDateTime),
) {
    let manifest_entries = vec![
        PageSourceManifestEntry {
            id: second_page_source_entry.id.clone(),
            chapter_id: page_source_fixture.chapter_info.id.clone(),
            index: 0,
        },
        PageSourceManifestEntry {
            id: page_source_fixture.page_source_entry.id.clone(),
            chapter_id: page_source_fixture.chapter_info.id.clone(),
            index: 1,
        },
    ];

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &ShiftPageSourceIndexesTemporary {
                chapter_id: &page_source_fixture.chapter_info.id,
            },
        )
        .await?;

        let page_source_infos = repo
            .step(
                context,
                &ApplyPageSourceManifest {
                    entries: &manifest_entries,
                },
            )
            .await?;

        assert_eq!(page_source_infos.len(), 2);

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let reordered_page_source_infos = repo
        .run(&ListPageSourceInfos {
            chapter_id: &page_source_fixture.chapter_info.id,
        })
        .await
        .unwrap();

    assert_page_source_order(
        &reordered_page_source_infos,
        [
            (&second_page_source_entry.id, new_created_at, (0, 0, 0)),
            (
                &page_source_fixture.page_source_entry.id,
                retained_created_at,
                (2, 1, 1),
            ),
        ],
    );

    let chapter_ids = vec![page_source_fixture.chapter_info.id.as_str()];

    let first_page_source_infos = repo
        .run(&ListFirstPageSourceInfos {
            chapter_ids: &chapter_ids,
        })
        .await
        .unwrap();

    let first_page_source_info = first_page_source_infos
        .iter()
        .find(|page_source_info| {
            page_source_info.chapter_id == page_source_fixture.chapter_info.id
        })
        .expect("first page info for the chapter");

    assert_eq!(first_page_source_info.id, second_page_source_entry.id);
}

// Force a manifest transaction failure and verify all original metadata survives.
async fn verify_manifest_rollback(
    repo: &RdbRepo,
    nucl: &RdbNucl<ReptRead>,
    page_source_fixture: &PageSourceFixture,
    second_page_source_entry: &PageSourceManifestEntry,
    (retained_created_at, new_created_at): (OffsetDateTime, OffsetDateTime),
) {
    let rollback_entries = vec![
        PageSourceManifestEntry {
            id: page_source_fixture.page_source_entry.id.clone(),
            chapter_id: page_source_fixture.chapter_info.id.clone(),
            index: 0,
        },
        PageSourceManifestEntry {
            id: second_page_source_entry.id.clone(),
            chapter_id: page_source_fixture.chapter_info.id.clone(),
            index: 1,
        },
    ];

    let rollback_result = nucl
        .coord(async |context| {
            //
            repo.step(
                context,
                &ShiftPageSourceIndexesTemporary {
                    chapter_id: &page_source_fixture.chapter_info.id,
                },
            )
            .await?;

            repo.step(
                context,
                &ApplyPageSourceManifest {
                    entries: &rollback_entries,
                },
            )
            .await?;

            Err::<(), BaseError>(BaseError::Unrecoverable {
                message: "force page-manifest rollback".into(),
            })
        })
        .await;

    assert!(rollback_result.is_err());

    let rolled_back_page_source_infos = repo
        .run(&ListPageSourceInfos {
            chapter_id: &page_source_fixture.chapter_info.id,
        })
        .await
        .unwrap();

    assert_page_source_order(
        &rolled_back_page_source_infos,
        [
            (&second_page_source_entry.id, new_created_at, (0, 0, 0)),
            (
                &page_source_fixture.page_source_entry.id,
                retained_created_at,
                (2, 1, 1),
            ),
        ],
    );
}

// Verify page lookup hides the tombstoned parent chapter.
async fn verify_tombstoned_page(
    shared: &RdbCore,
    repo: &RdbRepo,
    page_source_fixture: &PageSourceFixture,
) {
    let mut conn = shared.get().await.unwrap();

    diesel::update(
        t_chapter::table
            .filter(t_chapter::f_id.eq(&page_source_fixture.chapter_info.id)),
    )
    .set(t_chapter::f_deleted_at.eq(Some(time::OffsetDateTime::now_utc())))
    .execute(&mut conn)
    .await
    .unwrap();

    drop(conn);

    let page_source_error = repo
        .run(&GetPageSourceInfo {
            id: &page_source_fixture.page_source_entry.id,
        })
        .await
        .err()
        .unwrap();

    assert!(matches!(
        page_source_error,
        BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        }
    ));
}

// Verify every bounded page list rejects an overflowing stored manifest.
async fn verify_page_source_count_bound(
    repo: &RdbRepo,
    nucl: &RdbNucl<ReptRead>,
    page_source_fixture: &PageSourceFixture,
) {
    let overflow_entries = (2..=MAX_CHAPTER_PAGE_SOURCE_COUNT)
        .map(|index| PageSourceManifestEntry {
            id: [
                PREFIX,
                "page_source--73975336418",
                &format!("{:03}", &index),
            ]
            .concat(),
            chapter_id: page_source_fixture.chapter_info.id.clone(),
            index,
        })
        .collect::<Vec<_>>();

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &ApplyPageSourceManifest {
                entries: &overflow_entries,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let list_error = repo
        .run(&ListPageSourceInfos {
            chapter_id: &page_source_fixture.chapter_info.id,
        })
        .await
        .err()
        .unwrap();

    assert!(matches!(list_error, BaseError::Unrecoverable { .. }));

    let diff_error = repo
        .run(&ListPageSourceUnitDiffStats {
            chapter_id: &page_source_fixture.chapter_info.id,
        })
        .await
        .err()
        .unwrap();

    assert!(matches!(diff_error, BaseError::Unrecoverable { .. }));

    let excluded_error = nucl
        .coord(async |context| {
            repo.step(
                context,
                &ListPageSourceInfosExcluded {
                    chapter_id: &page_source_fixture.chapter_info.id,
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
pub async fn page_source_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let page_source_fixture = test_shared::seed_page(&shared, PREFIX).await;

    let config = AppConfig::default();

    let repo = RdbRepo::new(shared.clone(), config.repo);

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let retained_created_at =
        verify_page_source_counters(&repo, &nucl, &page_source_fixture).await;

    let (second_page_source_entry, new_created_at) =
        create_second_page(&repo, &nucl, &page_source_fixture).await;

    verify_manifest_reorder(
        &repo,
        &nucl,
        &page_source_fixture,
        &second_page_source_entry,
        (retained_created_at, new_created_at),
    )
    .await;

    verify_manifest_rollback(
        &repo,
        &nucl,
        &page_source_fixture,
        &second_page_source_entry,
        (retained_created_at, new_created_at),
    )
    .await;

    verify_tombstoned_page(&shared, &repo, &page_source_fixture).await;

    verify_page_source_count_bound(&repo, &nucl, &page_source_fixture).await;

    test_shared::cleanup(&shared, PREFIX).await.unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .unwrap();
}

/// Verifies original filename upsert, rollback, foreign keys, and page cascades.
/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
pub async fn raw_ident_roundtrip_uses_testcontainer(shared: RdbCore) {
    use crate::part::repo::oper::page_source::{
        DeletePageSources, ListPageSourceRawIdentInfos,
        UpdatePageSourceRawIdents,
    };
    use crate::shape::write::page_source::PageSourceRawIdentsReplShape;

    const RAW_PREFIX: &str = "rdb-test-page-raw-ident-";

    test_shared::reset(&shared, RAW_PREFIX).await;

    let fixture = test_shared::seed_page(&shared, RAW_PREFIX).await;

    let config = AppConfig::default();

    let repo = RdbRepo::new(shared.clone(), config.repo);

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let page_source_id = fixture.page_source_entry.id.as_str();

    let page_source_ids = [page_source_id];

    let mut created_at = None;

    for (repl, expected) in [
        (Some("原稿 01.JPG"), Some("原稿 01.JPG")),
        (Some("原稿 01.JPG"), Some("原稿 01.JPG")),
        (Some("renamed.png"), Some("renamed.png")),
        (None, None),
        (None, None),
    ] {
        let repls = PageSourceRawIdentsReplShape {
            idents: vec![(page_source_id, repl)],
        };

        nucl.coord(async |context| {
            repo.step(context, &UpdatePageSourceRawIdents { repl: &repls })
                .await
        })
        .await
        .unwrap();

        let infos = repo
            .run(&ListPageSourceRawIdentInfos {
                page_source_ids: &page_source_ids,
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

    let repls = PageSourceRawIdentsReplShape {
        idents: vec![
            (page_source_id, Some("rollback.png")),
            ("missing-page", Some("invalid.png")),
        ],
    };

    assert!(
        nucl.coord(async |context| repo
            .step(context, &UpdatePageSourceRawIdents { repl: &repls })
            .await)
            .await
            .is_err()
    );
    assert_raw_ident_absent(&repo, &page_source_ids).await;

    let repls = PageSourceRawIdentsReplShape {
        idents: vec![(page_source_id, Some("delete.png"))],
    };

    nucl.coord(async |context| {
        repo.step(context, &UpdatePageSourceRawIdents { repl: &repls })
            .await
    })
    .await
    .unwrap();

    let ids = [page_source_id.to_owned()];

    nucl.coord(async |context| {
        repo.step(context, &DeletePageSources::Ids { ids: &ids })
            .await
    })
    .await
    .unwrap();

    assert_raw_ident_absent(&repo, &page_source_ids).await;
    let empty = repo
        .run(&ListPageSourceRawIdentInfos {
            page_source_ids: &[],
        })
        .await
        .unwrap();

    assert!(empty.is_empty());

    test_shared::cleanup(&shared, RAW_PREFIX).await.unwrap();
}

async fn assert_raw_ident_absent(repo: &RdbRepo, page_source_ids: &[&str]) {
    use crate::part::repo::oper::page_source::ListPageSourceRawIdentInfos;

    let infos = repo
        .run(&ListPageSourceRawIdentInfos { page_source_ids })
        .await
        .unwrap();

    assert!(infos.is_empty());
}
