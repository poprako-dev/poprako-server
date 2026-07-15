// page_roundtrip_reads_test_database_url(SetPageUnitCounters, ListPageInfos)(positive): page repo persists, lists, and updates page counters in the local test database.

use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use crate::model::page::PageEntry;
use crate::model::unit::UnitCounters;
use crate::part::repo::oper::page::{
    CreatePages, ListFirstPageInfos, ListPageInfos, SetPageUnitCounters,
};
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::BaseError;

const PREFIX: &str = "rdb-test-page-domain-";

#[tokio::test]
async fn page_roundtrip_reads_test_database_url() {
    //
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let page_fixture = test_shared::seed_page(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let unit_counters = UnitCounters {
        total_unit_count: 2,
        translated_unit_count: 1,
        proofread_unit_count: 1,
    };

    drive
        .coord(async |context| {
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
        .run(&ListPageInfos::Chapter {
            chapter_id: &page_fixture.chapter_entry.id,
            offset: 0,
            limit: 10,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(page_infos.len(), 1);

    assert_eq!(page_infos[0].total_unit_count, 2);

    let second_page_entry = PageEntry {
        id: format!("{}page-later", PREFIX),
        chapter_id: page_fixture.chapter_entry.id.clone(),
        index: 1,
        image_key: None,
        image_version: 0,
    };

    drive
        .coord(async |context| {
            //
            repo.step(
                context,
                &CreatePages {
                    entries: &[second_page_entry],
                },
            )
            .await?;

            Ok::<(), BaseError>(())
        })
        .await
        .ok()
        .unwrap();

    let chapter_ids = vec![page_fixture.chapter_entry.id.clone()];

    let first_page_infos = repo
        .run(&ListFirstPageInfos {
            chapter_ids: &chapter_ids,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(
        first_page_infos[&page_fixture.chapter_entry.id].id,
        page_fixture.page_entry.id
    );

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
