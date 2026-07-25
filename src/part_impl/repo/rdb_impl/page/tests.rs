// page_roundtrip_uses_testcontainer(SetPageUnitCounters, ReservePageImage, ListPageInfos)(positive): page repo persists, returns the replaced image key, and updates page counters in an isolated PostgreSQL container.

use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use crate::model::page::PageEntry;
use crate::model::unit::UnitCounters;
use crate::part::repo::oper::page::{CreatePages, GetPageInfo, ListFirstPageInfos, ListPageInfos, ReservePageImage, SetPageUnitCounters};
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::part_impl::shared::RdbCore;
use crate::result::BaseError;
use crate::value::image::ImageExt;

const PREFIX: &str = "rdb-test-page-domain-";

/// Verifies page roundtrip via testcontainers.
/// Verifies page roundtrip via testcontainers.
pub async fn page_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
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
        image_key: Some("page/previous.png".into()),
        image_version: 1,
        image_hash: Default::default(),
        image_ext: ImageExt::Jpg,
    };

    let second_page_id = second_page_entry.id.clone();

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

    let image_reservation = drive
        .coord(async |context| {
            repo.step(
                context,
                &ReservePageImage {
                    id: &second_page_id,
                    file_ext: "jpg",
                },
            )
            .await
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(
        image_reservation.prev_object_key,
        Some("page/previous.png".into())
    );

    assert_eq!(image_reservation.image_version, 2);

    let replaced_page_info = repo
        .run(&GetPageInfo {
            id: &second_page_id,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(
        replaced_page_info.image_key,
        Some(image_reservation.object_key)
    );

    assert!(!replaced_page_info.image_uploaded);

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
