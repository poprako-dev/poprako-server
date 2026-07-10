// page_roundtrip_reads_test_database_url(PageStep)(positive): page repo persists, lists, and updates page counters in the local test database.

use super::*;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use poprako_util::page::Page;

use crate::model::unit::UnitCounters;
use crate::part::repo::step::page::PageStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::RegularError;
use crate::util::DeriveTransactional as _;

const PREFIX: &str = "rdb-test-page-domain-";

#[tokio::test]
async fn page_roundtrip_reads_test_database_url() {
    //
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let page_fixture = test_shared::seed_page(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let transactional_repo = repo.derive_transactional().await;

    let unit_counters = UnitCounters {
        total_unit_count: 2,
        translated_unit_count: 1,
        proofread_unit_count: 1,
    };

    drive
        .with_context(async |context| {
            //
            Advance::advance(
                &transactional_repo,
                context,
                &PageStep::set_unit_counters(
                    &page_fixture.page_form.id,
                    unit_counters,
                ),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    let page = Page {
        offset: 0,
        limit: 10,
    };

    let page_infos = Execute::execute(
        &repo,
        &PageStep::list_infos_by_chapter_id(
            &page_fixture.chapter_form.id,
            page,
        ),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(page_infos.len(), 1);

    assert_eq!(page_infos[0].total_unit_count, 2);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
