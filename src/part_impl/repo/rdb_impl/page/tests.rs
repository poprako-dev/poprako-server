// page_roundtrip_reads_test_database_url(SetPageUnitCounters, ListPageInfos)(positive): page repo persists, lists, and updates page counters in the local test database.

use super::*;

use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use crate::model::unit::UnitCounters;
use crate::part::repo::oper::page::{ListPageInfos, SetPageUnitCounters};
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::RegularError;

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
            repo.step(
                context,
                &SetPageUnitCounters {
                    id: &page_fixture.page_entry.id,
                    counters: unit_counters,
                },
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    let list_page_infos = ListPageInfos::Chapter {
        chapter_id: &page_fixture.chapter_entry.id,
        offset: 0,
        limit: 10,
    };

    let page_infos = repo.run(&list_page_infos).await.ok().unwrap();

    assert_eq!(page_infos.len(), 1);

    assert_eq!(page_infos[0].total_unit_count, 2);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
