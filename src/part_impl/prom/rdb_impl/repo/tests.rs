// completed_message_purge_preserves_non_completed_records(PurgeCompleted)(positive): expired completed records are purged while recent completed, pending, and dead records remain.
// poll_pending_selects_one_visible_message_per_idle_topic(PollPending)(positive): polling is fair across topics and skips topics with processing work.
// retry_message_allows_later_topic_message_to_advance(RetryMessage)(positive): delayed retries are equivalent to re-enqueueing behind visible work.

use super::*;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::Duration;

use crate::part_impl::prom::rdb_impl::entity::LocalMessageEntry;
use crate::part_impl::prom::rdb_impl::test_shared;
use crate::part_impl::repo::rdb_impl::{RdbRepo, schema};
use crate::part_impl::shared::{RdbContext, RdbCore};

const PREFIX: &str = "rdb-test-prom-purge-";
const POLL_PREFIX: &str = "rdb-test-prom-poll-";

fn local_message_entry(
    id: &'static str,
    topic: &'static str,
    status: LocalMessageStatus,
    created_at: OffsetDateTime,
) -> LocalMessageEntry<'static> {
    LocalMessageEntry {
        f_id: id,
        f_topic: topic,
        f_status: status,
        f_payload: serde_json::json!({}),
        f_visible_at: created_at,
        f_created_at: created_at,
        f_updated_at: created_at,
    }
}

pub async fn poll_pending_selects_one_visible_message_per_idle_topic(
    shared: RdbCore,
) {
    test_shared::reset(&shared, POLL_PREFIX).await;

    let now = OffsetDateTime::now_utc();

    let entries = [
        local_message_entry(
            "rdb-test-prom-poll-image-first",
            "rdb-test-prom-poll-image",
            LocalMessageStatus::Pending,
            now - Duration::minutes(5),
        ),
        local_message_entry(
            "rdb-test-prom-poll-image-second",
            "rdb-test-prom-poll-image",
            LocalMessageStatus::Pending,
            now - Duration::minutes(4),
        ),
        local_message_entry(
            "rdb-test-prom-poll-invitation",
            "rdb-test-prom-poll-invitation",
            LocalMessageStatus::Pending,
            now - Duration::minutes(3),
        ),
        local_message_entry(
            "rdb-test-prom-poll-chapter-processing",
            "rdb-test-prom-poll-chapter",
            LocalMessageStatus::Processing,
            now - Duration::minutes(2),
        ),
        local_message_entry(
            "rdb-test-prom-poll-chapter-pending",
            "rdb-test-prom-poll-chapter",
            LocalMessageStatus::Pending,
            now - Duration::minutes(1),
        ),
    ];

    let mut conn = shared.get().await.ok().unwrap();

    diesel::insert_into(schema::t_local_message::table)
        .values(&entries)
        .execute(&mut conn)
        .await
        .ok()
        .unwrap();

    let repo = RdbPromRepo::new(RdbRepo::new(shared.clone()));

    let mut context = RdbContext::new(shared.get().await.ok().unwrap());

    let mut rows = repo.step(&mut context, &PollPending).await.ok().unwrap();

    rows.retain(|row| row.f_id.starts_with(POLL_PREFIX));

    rows.sort_by(|left, right| left.f_id.cmp(&right.f_id));

    assert_eq!(
        rows.into_iter().map(|row| row.f_id).collect::<Vec<_>>(),
        vec![
            "rdb-test-prom-poll-image-first".to_string(),
            "rdb-test-prom-poll-invitation".to_string(),
        ]
    );

    test_shared::cleanup(&shared, POLL_PREFIX)
        .await
        .ok()
        .unwrap();

    test_shared::assert_no_leftovers(&shared, POLL_PREFIX)
        .await
        .ok()
        .unwrap();
}

pub async fn retry_message_allows_later_topic_message_to_advance(
    shared: RdbCore,
) {
    test_shared::reset(&shared, POLL_PREFIX).await;

    let now = OffsetDateTime::now_utc();

    let entries = [
        local_message_entry(
            "rdb-test-prom-poll-image-retry",
            "rdb-test-prom-poll-retry-image",
            LocalMessageStatus::Processing,
            now - Duration::minutes(2),
        ),
        local_message_entry(
            "rdb-test-prom-poll-image-next",
            "rdb-test-prom-poll-retry-image",
            LocalMessageStatus::Pending,
            now - Duration::minutes(1),
        ),
    ];

    let mut conn = shared.get().await.ok().unwrap();

    diesel::insert_into(schema::t_local_message::table)
        .values(&entries)
        .execute(&mut conn)
        .await
        .ok()
        .unwrap();

    let repo = RdbPromRepo::new(RdbRepo::new(shared.clone()));

    let retry_visible_at = now + Duration::minutes(5);

    let mut context = RdbContext::new(shared.get().await.ok().unwrap());

    repo.step(
        &mut context,
        &RetryMessage::new(
            "rdb-test-prom-poll-image-retry",
            "temporary failure",
            &retry_visible_at,
        ),
    )
    .await
    .ok()
    .unwrap();

    let rows = repo.step(&mut context, &PollPending).await.ok().unwrap();

    let rows = rows
        .into_iter()
        .filter(|row| row.f_id.starts_with(POLL_PREFIX))
        .collect::<Vec<_>>();

    assert_eq!(rows.len(), 1);

    assert_eq!(rows[0].f_id, "rdb-test-prom-poll-image-next");

    test_shared::cleanup(&shared, POLL_PREFIX)
        .await
        .ok()
        .unwrap();

    test_shared::assert_no_leftovers(&shared, POLL_PREFIX)
        .await
        .ok()
        .unwrap();
}

pub async fn completed_message_purge_preserves_non_completed_records(
    shared: RdbCore,
) {
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
