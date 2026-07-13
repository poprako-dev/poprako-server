// completed_message_purge_preserves_non_completed_records(PurgeCompleted)(positive): expired completed records are purged while recent completed, pending, and dead records remain.

use super::*;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::Duration;

use crate::part_impl::prom::rdb_impl::entity::LocalMessageEntry;
use crate::part_impl::repo::rdb_impl::{RdbRepo, schema, test_shared};
use crate::part_impl::shared::RdbContext;

const PREFIX: &str = "rdb-test-prom-purge-";

#[tokio::test]
async fn completed_message_purge_preserves_non_completed_records() {
    //
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let now = OffsetDateTime::now_utc();

    let stale_completed_entry = LocalMessageEntry {
        f_id: "rdb-test-prom-purge-stale-completed",
        f_topic: "image",
        f_status: LocalMessageStatus::Completed,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(8),
        f_created_at: now - Duration::days(8),
        f_updated_at: now - Duration::days(8),
    };

    let recent_completed_entry = LocalMessageEntry {
        f_id: "rdb-test-prom-purge-recent-completed",
        f_topic: "image",
        f_status: LocalMessageStatus::Completed,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(1),
        f_created_at: now - Duration::days(1),
        f_updated_at: now - Duration::days(1),
    };

    let pending_entry = LocalMessageEntry {
        f_id: "rdb-test-prom-purge-pending",
        f_topic: "image",
        f_status: LocalMessageStatus::Pending,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(8),
        f_created_at: now - Duration::days(8),
        f_updated_at: now - Duration::days(8),
    };

    let dead_entry = LocalMessageEntry {
        f_id: "rdb-test-prom-purge-dead",
        f_topic: "image",
        f_status: LocalMessageStatus::Dead,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(8),
        f_created_at: now - Duration::days(8),
        f_updated_at: now - Duration::days(8),
    };

    let mut conn = shared.get().await.ok().unwrap();

    diesel::insert_into(schema::t_local_message::table)
        .values(&[
            stale_completed_entry,
            recent_completed_entry,
            pending_entry,
            dead_entry,
        ])
        .execute(&mut conn)
        .await
        .ok()
        .unwrap();

    let repo = RdbPromRepo::new(RdbRepo::new(shared.clone()));

    let before = now - Duration::days(7);

    let mut context = RdbContext::new(shared.get().await.ok().unwrap());

    let purged_count = repo
        .step(&mut context, &PurgeCompleted::new(&before))
        .await
        .ok()
        .unwrap();

    assert_eq!(purged_count, 1);

    let remaining_ids: Vec<String> = schema::t_local_message::table
        .filter(schema::t_local_message::f_id.like(format!("{}%", PREFIX)))
        .order_by(schema::t_local_message::f_id.asc())
        .select(schema::t_local_message::f_id)
        .load(&mut conn)
        .await
        .ok()
        .unwrap();

    assert_eq!(
        remaining_ids,
        vec![
            "rdb-test-prom-purge-dead".to_string(),
            "rdb-test-prom-purge-pending".to_string(),
            "rdb-test-prom-purge-recent-completed".to_string(),
        ]
    );

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
