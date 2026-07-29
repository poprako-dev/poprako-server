use super::*;

use crate::shared::test_rdb::start;

#[tokio::test]
#[serial_test::serial(prom_rdb)]
async fn prom_rdb_impls_use_testcontainer() {
    //
    let test_rdb = start().await;

    let shared = test_rdb.core();

    handler::tests::image_payloads_from_rdb_dispatch(shared.clone()).await;

    repo::tests::poll_pending_selects_one_visible_message_per_idle_topic(
        shared.clone(),
    )
    .await;

    repo::tests::retry_message_allows_later_topic_message_to_advance(
        shared.clone(),
    )
    .await;

    repo::tests::stale_attempt_finalization_preserves_newer_lease(
        shared.clone(),
    )
    .await;

    repo::tests::completed_message_purge_preserves_non_completed_records(
        shared,
    )
    .await;

    drop(test_rdb);
}
