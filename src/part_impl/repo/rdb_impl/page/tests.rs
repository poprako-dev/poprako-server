// page_roundtrip_uses_testcontainer(SetPageUnitCounters, ListPageInfos)(positive): page repo persists and updates page counters in an isolated PostgreSQL container.

use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use poprako_rdb_core::RdbCore;

use crate::model::read::proj::unit::UnitCountMetrics;
use crate::model::write::page::PageEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page::{
    CreatePages, ListFirstPageInfos, ListPageInfos, SetPageUnitCounters,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::BaseError;

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

    let second_page_entry = PageEntry {
        id: format!("{}page-later", PREFIX),
        chapter_id: page_fixture.chapter_entry.id.clone(),
        index: 1,
    };

    nucl.coord(async |context| {
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

    let first_page_info = first_page_infos
        .iter()
        .find(|page_info| page_info.chapter_id == page_fixture.chapter_entry.id)
        .expect("first page info for the chapter");

    assert_eq!(first_page_info.id, page_fixture.page_entry.id);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
