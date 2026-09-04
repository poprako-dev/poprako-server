// page_roundtrip_uses_testcontainer(SetPageUnitCounters, ListPageInfos)(positive): page repo persists and updates page counters in an isolated PostgreSQL container.

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use poprako_rdb_core::RdbCore;

use crate::model::read::proj::unit::UnitCountMetrics;
use crate::model::write::page::PageManifestEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page::{
    ApplyPageManifest, GetPageInfo, ListEdittedDiffPageIds, ListFirstPageInfos,
    ListPageInfos, ListPageInfosExcluded, SetPageUnitCounters,
    ShiftPageIndexesTemporary,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::schema::t_chapter;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::{BaseError, ExpectedVariant};
use crate::value::page::MAX_CHAPTER_PAGE_COUNT;

const PREFIX: &str = "rdb-test-page-domain-";

/// Verifies page roundtrip via testcontainers.
/// Verifies page roundtrip via testcontainers.
pub async fn page_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let page_fixture = test_shared::seed_page(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let unit_counters = UnitCountMetrics {
        total: 2,
        translated: 1,
        proofread: 1,
    };

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &SetPageUnitCounters {
                id: &page_fixture.page_entry.id,
                counters: unit_counters,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let page_infos = repo
        .run(&ListPageInfos {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(page_infos.len(), 1);

    assert_eq!(page_infos[0].total_unit_count, 2);

    assert_eq!(page_infos[0].translated_unit_count, 1);

    assert_eq!(page_infos[0].proofread_unit_count, 1);

    let retained_created_at = page_infos[0].created_at;

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
                    message: "page creation returned no row".into(),
                }
            })
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(new_page_info.total_unit_count, 0);

    assert_eq!(new_page_info.translated_unit_count, 0);

    assert_eq!(new_page_info.proofread_unit_count, 0);

    let new_created_at = new_page_info.created_at;

    assert!(new_created_at <= time::OffsetDateTime::now_utc());

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
    .ok()
    .unwrap();

    let reordered_page_infos = repo
        .run(&ListPageInfos {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(reordered_page_infos[0].id, second_page_entry.id);

    assert_eq!(reordered_page_infos[0].total_unit_count, 0);

    assert_eq!(reordered_page_infos[0].translated_unit_count, 0);

    assert_eq!(reordered_page_infos[0].proofread_unit_count, 0);

    assert_eq!(reordered_page_infos[0].created_at, new_created_at);

    assert_eq!(reordered_page_infos[1].id, page_fixture.page_entry.id);

    assert_eq!(reordered_page_infos[1].total_unit_count, 2);

    assert_eq!(reordered_page_infos[1].translated_unit_count, 1);

    assert_eq!(reordered_page_infos[1].proofread_unit_count, 1);

    assert_eq!(reordered_page_infos[1].created_at, retained_created_at);

    let chapter_ids = vec![page_fixture.chapter_entry.id.as_str()];

    let first_page_infos = repo
        .run(&ListFirstPageInfos {
            chapter_ids: &chapter_ids,
        })
        .await
        .ok()
        .unwrap();

    let first_page_info = first_page_infos
        .iter()
        .find(|page_info| page_info.chapter_id == page_fixture.chapter_entry.id)
        .expect("first page info for the chapter");

    assert_eq!(first_page_info.id, second_page_entry.id);

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
                message: "force page-manifest rollback".into(),
            })
        })
        .await;

    assert!(rollback_result.is_err());

    let rolled_back_page_infos = repo
        .run(&ListPageInfos {
            chapter_id: &page_fixture.chapter_entry.id,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(rolled_back_page_infos[0].id, second_page_entry.id);

    assert_eq!(rolled_back_page_infos[0].created_at, new_created_at);

    assert_eq!(rolled_back_page_infos[0].total_unit_count, 0);

    assert_eq!(rolled_back_page_infos[0].translated_unit_count, 0);

    assert_eq!(rolled_back_page_infos[0].proofread_unit_count, 0);

    assert_eq!(rolled_back_page_infos[1].id, page_fixture.page_entry.id);

    assert_eq!(rolled_back_page_infos[1].created_at, retained_created_at);

    assert_eq!(rolled_back_page_infos[1].total_unit_count, 2);

    assert_eq!(rolled_back_page_infos[1].translated_unit_count, 1);

    assert_eq!(rolled_back_page_infos[1].proofread_unit_count, 1);

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
        .run(&ListEdittedDiffPageIds {
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

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
