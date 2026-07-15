// unit_roundtrip_reads_test_database_url(UnitRepo)(positive): unit repo creates, saves, restores, reindexes, and lists units.
// unit_roundtrip_reads_test_database_url(UnitRepo)(negative): unit create rejects an existing server id without mutation.

use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use poprako_util::page::Page;

use crate::model::unit::{UnitContent, UnitIndexUpdate};
use crate::part::repo::oper::unit::{
    CreateUnit, ListUnitInfos, SaveUnit, UpdateUnitIndexes,
};
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::{BaseResult, accept};

const PREFIX: &str = "rdb-test-unit-domain-";

fn unit_payload(text: Option<&str>, proofread: bool) -> UnitContent {
    UnitContent {
        is_bubble: true,
        is_proofread: proofread,
        x_coord: 1.0,
        y_coord: 2.0,
        translated_text: text.map(Into::into),
        last_translator_id: None,
        proofread_text: None,
        last_proofreader_id: None,
    }
}

#[tokio::test]
async fn unit_roundtrip_reads_test_database_url() {
    //
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let page_fixture = test_shared::seed_page(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let nucl = RdbDrive::new(shared.clone());

    let unit_id = format!("{}unit", PREFIX);

    let restored_unit_id = format!("{}restored-unit", PREFIX);

    let create_unit_payload = unit_payload(Some("translated"), false);

    let save_unit_payload = unit_payload(Some("translated"), true);

    let restored_unit_payload = unit_payload(Some("restored"), false);

    let unit_index_updates = [UnitIndexUpdate {
        id: unit_id.clone(),
        index: 5,
    }];

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &CreateUnit {
                page_id: &page_fixture.page_entry.id,
                id: &unit_id,
                payload: &create_unit_payload,
            },
        )
        .await?;

        repo.step(
            context,
            &SaveUnit {
                page_id: &page_fixture.page_entry.id,
                id: &unit_id,
                payload: &save_unit_payload,
            },
        )
        .await?;

        repo.step(
            context,
            &SaveUnit {
                page_id: &page_fixture.page_entry.id,
                id: &restored_unit_id,
                payload: &restored_unit_payload,
            },
        )
        .await?;

        repo.step(
            context,
            &UpdateUnitIndexes {
                page_id: &page_fixture.page_entry.id,
                updates: &unit_index_updates,
            },
        )
        .await?;

        accept(())
    })
    .await
    .ok()
    .unwrap();

    let duplicate_create_result = nucl
        .coord(async |context| {
            repo.step(
                context,
                &CreateUnit {
                    page_id: &page_fixture.page_entry.id,
                    id: &unit_id,
                    payload: &create_unit_payload,
                },
            )
            .await
        })
        .await;

    assert!(duplicate_create_result.is_err());

    let unit_infos = repo
        .run(&ListUnitInfos::Page {
            page_id: &page_fixture.page_entry.id,
            page: Page {
                offset: 0,
                limit: 10,
            },
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(unit_infos.len(), 2);

    let saved_unit_info = unit_infos
        .iter()
        .find(|unit_info| unit_info.id == unit_id)
        .unwrap();

    assert_eq!(saved_unit_info.index, 5);

    assert!(saved_unit_info.is_proofread);

    let restored_unit_info = unit_infos
        .iter()
        .find(|unit_info| unit_info.id == restored_unit_id)
        .unwrap();

    assert_eq!(
        restored_unit_info.translated_text.as_deref(),
        Some("restored")
    );

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
