// unit_roundtrip_reads_test_database_url(UnitStep)(positive): unit repo creates, saves, reindexes, and lists units in the local test database.

use super::*;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::page::Page;

use crate::model::unit::{UnitIndexUpdate, UnitOper, UnitPayload};
use crate::part::repo::step::unit::UnitStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::RegularError;
use crate::util::DeriveTransactional as _;

const PREFIX: &str = "rdb-test-unit-domain-";

fn unit_payload(text: Option<&str>, proofread: bool) -> UnitPayload {
    UnitPayload {
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

    let drive = RdbDrive::new(shared.clone());

    let transactional_repo = repo.derive_transactional().await;

    let unit_id = format!("{}unit", PREFIX);

    let create_unit_oper = UnitOper::Save {
        local_id: Some("local-1".into()),
        id: Some(unit_id.clone()),
        payload: unit_payload(Some("translated"), false),
        before_id: None,
    };

    let save_unit_oper = UnitOper::Save {
        local_id: None,
        id: Some(unit_id.clone()),
        payload: unit_payload(Some("translated"), true),
        before_id: None,
    };

    let unit_index_updates = [UnitIndexUpdate {
        id: unit_id,
        index: 5,
    }];

    drive
        .with_context(async |context| {
            //
            Advance::advance(
                &transactional_repo,
                context,
                &UnitStep::save_info(
                    &page_fixture.page_form.id,
                    &create_unit_oper,
                ),
            )
            .await?;

            Advance::advance(
                &transactional_repo,
                context,
                &UnitStep::save_info(
                    &page_fixture.page_form.id,
                    &save_unit_oper,
                ),
            )
            .await?;

            Advance::advance(
                &transactional_repo,
                context,
                &UnitStep::update_indexes_by_page_id(
                    &page_fixture.page_form.id,
                    &unit_index_updates,
                ),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    let unit_infos = Execute::execute(
        &repo,
        &UnitStep::list_infos_by_page_id(
            &page_fixture.page_form.id,
            Page {
                offset: 0,
                limit: 10,
            },
        ),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(unit_infos.len(), 1);

    assert_eq!(unit_infos[0].index, 5);

    assert!(unit_infos[0].is_proofread);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
