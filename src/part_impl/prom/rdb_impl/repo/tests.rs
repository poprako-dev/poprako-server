// completed_message_purge_preserves_non_completed_records(PurgeCompleted)(positive): expired completed records are purged while recent completed, pending, and dead records remain.
// poll_pending_selects_one_visible_message_per_idle_topic(PollPending)(positive): polling is fair across topics and skips topics with processing work.
// retry_message_allows_later_topic_message_to_advance(RetryMessage)(positive): delayed retries are equivalent to re-enqueueing behind visible work.
// stale_attempt_finalization_preserves_newer_lease(CompleteMessage/RetryMessage/FailMessage)(negative): an expired worker lease cannot finalize a newer processing attempt or overwrite Dead.

use super::*;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::Duration;

use crate::part::nucl::RepeatableRead;
use crate::part_impl::prom::rdb_impl::entity::LocalMessageEntryRow;
use crate::part_impl::prom::rdb_impl::test_shared;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::shared::{RdbContext, RdbCore};

// Constant definition for `PREFIX`.
const PREFIX: &str = "rdb-test-prom-purge-";
// Constant definition for `POLL_PREFIX`.
const POLL_PREFIX: &str = "rdb-test-prom-poll-";
// Constant definition for `LEASE_PREFIX`.
const LEASE_PREFIX: &str = "rdb-test-prom-lease-";

/// Verifies polling is fair across topics and skips topics with processing
/// work.
pub async fn poll_pending_selects_one_visible_message_per_idle_topic(
    shared: RdbCore,
) {
    //
    // Internal state field test_shared.
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

    diesel::insert_into(t_local_message::table)
        .values(&entries)
        .execute(&mut conn)
        .await
        .ok()
        .unwrap();

    let repo = RdbPromRepo::new(HybRepo::new(shared.clone()));

    let mut context =
        RdbContext::<RepeatableRead>::new(shared.get().await.ok().unwrap());

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

/// Verifies delayed retries are equivalent to re-enqueueing behind visible
/// work.
pub async fn retry_message_allows_later_topic_message_to_advance(
    shared: RdbCore,
) {
    //
    // Internal state field test_shared.
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

    diesel::insert_into(t_local_message::table)
        .values(&entries)
        .execute(&mut conn)
        .await
        .ok()
        .unwrap();

    let repo = RdbPromRepo::new(HybRepo::new(shared.clone()));

    let retry_visible_at = now + Duration::minutes(5);

    let mut context =
        RdbContext::<RepeatableRead>::new(shared.get().await.ok().unwrap());

    repo.step(
        &mut context,
        &RetryMessage::new(
            "rdb-test-prom-poll-image-retry",
            0,
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

/// Verifies every task-finalization operation is fenced by Processing status
/// and the worker attempt lease.
pub async fn stale_attempt_finalization_preserves_newer_lease(shared: RdbCore) {
    //
    // Internal state field test_shared.
    test_shared::reset(&shared, LEASE_PREFIX).await;

    let now = OffsetDateTime::now_utc();

    let entries = [
        local_message_entry(
            "rdb-test-prom-lease-complete",
            "rdb-test-prom-lease-topic",
            LocalMessageStatus::Processing,
            now,
        ),
        local_message_entry(
            "rdb-test-prom-lease-retry",
            "rdb-test-prom-lease-topic",
            LocalMessageStatus::Processing,
            now,
        ),
        local_message_entry(
            "rdb-test-prom-lease-fail",
            "rdb-test-prom-lease-topic",
            LocalMessageStatus::Dead,
            now,
        ),
    ];

    let mut conn = shared.get().await.ok().unwrap();

    diesel::insert_into(t_local_message::table)
        .values(&entries)
        .execute(&mut conn)
        .await
        .ok()
        .unwrap();

    diesel::update(
        t_local_message::table
            .filter(t_local_message::f_id.like(format!("{}%", LEASE_PREFIX))),
    )
    .set(t_local_message::f_lease.eq(1_i64))
    .execute(&mut conn)
    .await
    .ok()
    .unwrap();

    let repo = RdbPromRepo::new(HybRepo::new(shared.clone()));

    let retry_visible_at = now + Duration::minutes(5);

    let mut context =
        RdbContext::<RepeatableRead>::new(shared.get().await.ok().unwrap());

    repo.step(
        &mut context,
        &CompleteMessage::new("rdb-test-prom-lease-complete", 0),
    )
    .await
    .ok()
    .unwrap();

    repo.step(
        &mut context,
        &RetryMessage::new(
            "rdb-test-prom-lease-retry",
            0,
            "stale retry",
            &retry_visible_at,
        ),
    )
    .await
    .ok()
    .unwrap();

    repo.step(
        &mut context,
        &FailMessage::new("rdb-test-prom-lease-fail", 0, "stale fail"),
    )
    .await
    .ok()
    .unwrap();

    let rows: Vec<(String, String, i64, i64)> = t_local_message::table
        .filter(t_local_message::f_id.like(format!("{}%", LEASE_PREFIX)))
        .order_by(t_local_message::f_id.asc())
        .select((
            t_local_message::f_id,
            t_local_message::f_status,
            t_local_message::f_retried_count,
            t_local_message::f_lease,
        ))
        .load(&mut conn)
        .await
        .ok()
        .unwrap();

    assert_eq!(
        rows,
        vec![
            (
                "rdb-test-prom-lease-complete".to_string(),
                LocalMessageStatus::Processing.as_str().to_string(),
                0,
                1,
            ),
            (
                "rdb-test-prom-lease-fail".to_string(),
                LocalMessageStatus::Dead.as_str().to_string(),
                0,
                1,
            ),
            (
                "rdb-test-prom-lease-retry".to_string(),
                LocalMessageStatus::Processing.as_str().to_string(),
                0,
                1,
            ),
        ]
    );

    test_shared::cleanup(&shared, LEASE_PREFIX)
        .await
        .ok()
        .unwrap();

    test_shared::assert_no_leftovers(&shared, LEASE_PREFIX)
        .await
        .ok()
        .unwrap();
}

/// Verifies expired completed records are purged while recent completed,
/// pending, and dead records remain.
pub async fn completed_message_purge_preserves_non_completed_records(
    shared: RdbCore,
) {
    //
    // Internal state field test_shared.
    test_shared::reset(&shared, PREFIX).await;

    let now = OffsetDateTime::now_utc();

    let stale_completed_entry = LocalMessageEntryRow {
        f_id: "rdb-test-prom-purge-stale-completed",
        f_topic: "image",
        f_status: LocalMessageStatus::Completed,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(8),
        f_created_at: now - Duration::days(8),
        f_updated_at: now - Duration::days(8),
    };

    let recent_completed_entry = LocalMessageEntryRow {
        f_id: "rdb-test-prom-purge-recent-completed",
        f_topic: "image",
        f_status: LocalMessageStatus::Completed,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(1),
        f_created_at: now - Duration::days(1),
        f_updated_at: now - Duration::days(1),
    };

    let pending_entry = LocalMessageEntryRow {
        f_id: "rdb-test-prom-purge-pending",
        f_topic: "image",
        f_status: LocalMessageStatus::Pending,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(8),
        f_created_at: now - Duration::days(8),
        f_updated_at: now - Duration::days(8),
    };

    let dead_entry = LocalMessageEntryRow {
        f_id: "rdb-test-prom-purge-dead",
        f_topic: "image",
        f_status: LocalMessageStatus::Dead,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(8),
        f_created_at: now - Duration::days(8),
        f_updated_at: now - Duration::days(8),
    };

    let stale_dead_entry = LocalMessageEntryRow {
        f_id: "rdb-test-prom-purge-stale-dead",
        f_topic: "image",
        f_status: LocalMessageStatus::Dead,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(31),
        f_created_at: now - Duration::days(31),
        f_updated_at: now - Duration::days(31),
    };

    let mut conn = shared.get().await.ok().unwrap();

    diesel::insert_into(t_local_message::table)
        .values(&[
            stale_completed_entry,
            recent_completed_entry,
            pending_entry,
            dead_entry,
            stale_dead_entry,
        ])
        .execute(&mut conn)
        .await
        .ok()
        .unwrap();

    let repo = RdbPromRepo::new(HybRepo::new(shared.clone()));

    let completed_before = now - Duration::days(7);

    let dead_before = now - Duration::days(30);

    let mut context =
        RdbContext::<RepeatableRead>::new(shared.get().await.ok().unwrap());

    let purged_count = repo
        .step(
            &mut context,
            &PurgeCompleted::new(&completed_before, &dead_before),
        )
        .await
        .ok()
        .unwrap();

    assert_eq!(purged_count, 2);

    let remaining_ids: Vec<String> = t_local_message::table
        .filter(t_local_message::f_id.like(format!("{}%", PREFIX)))
        .order_by(t_local_message::f_id.asc())
        .select(t_local_message::f_id)
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

// Internal implementation of `local_message_entry`.
fn local_message_entry(
    id: &'static str,
    topic: &'static str,
    status: LocalMessageStatus,
    created_at: OffsetDateTime,
) -> LocalMessageEntryRow<'static> {
    LocalMessageEntryRow {
        f_id: id,
        f_topic: topic,
        f_status: status,
        f_payload: serde_json::json!({}),
        f_visible_at: created_at,
        f_created_at: created_at,
        f_updated_at: created_at,
    }
}
