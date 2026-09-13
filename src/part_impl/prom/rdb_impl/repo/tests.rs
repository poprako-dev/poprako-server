// dead_message_purge_preserves_pending_records(PurgeDead)(positive): expired dead records are purged while pending and recent dead records remain.
// claim_pending_selects_one_visible_message_per_idle_topic(ClaimPending)(positive): polling is fair across topics and skips topics with processing work.
// retry_message_allows_later_topic_message_to_advance(RetryMessage)(positive): delayed retries are equivalent to re-enqueueing behind visible work.
// wait_message_preserves_retry_budget(RetryMessage)(positive): waiting for external state returns the task to Pending without incrementing its retry counter.
// stale_attempt_finalization_preserves_recreated_task(CompleteMessage/RetryMessage/FailMessage)(negative): an expired worker claim_token cannot finalize a newer processing attempt or overwrite Dead.

use super::*;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::Duration;

use poprako_rdb_core::RdbCore;

use crate::part::nucl::{ReptRead, Serial};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::prom::rdb_impl::entity::LocalMessageEntryRow;
use crate::part_impl::prom::rdb_impl::test_shared;
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::shared::RdbContext;
use poprako_orchestra::{Nucl as _, OperStep as _};

// Constant definition for `PREFIX`.
const PREFIX: &str = "rdb-test-prom-purge-";
// Constant definition for `POLL_PREFIX`.
const POLL_PREFIX: &str = "rdb-test-prom-poll-";
// Constant definition for `LEASE_PREFIX`.
const LEASE_PREFIX: &str = "rdb-test-prom-claim_token-";
// Constant definition for `WAIT_PREFIX`.
const WAIT_PREFIX: &str = "rdb-test-prom-wait-";

/// Verifies claiming is fair across topics and skips topics with processing
/// work.
pub async fn claim_pending_selects_one_visible_message_per_idle_topic(
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

    let repo = RdbPromRepo::new();

    let mut rows = RdbNucl::<Serial>::new(shared.clone())
        .coord(async |context| {
            ClaimPending::new(4).step_on(&repo, context).await
        })
        .await
        .unwrap();

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

    let repo = RdbPromRepo::new();

    let retry_visible_at = now + Duration::minutes(5);

    let mut context =
        RdbContext::<ReptRead>::new(shared.get().await.ok().unwrap());

    repo.step(
        &mut context,
        &RetryMessage::new(
            "rdb-test-prom-poll-image-retry",
            Uuid::from_u128(1),
            "temporary failure",
            &retry_visible_at,
            1,
        ),
    )
    .await
    .ok()
    .unwrap();

    let rows = RdbNucl::<Serial>::new(shared.clone())
        .coord(async |context| {
            ClaimPending::new(4).step_on(&repo, context).await
        })
        .await
        .unwrap();

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

/// Verifies waiting for external state preserves the failure retry budget.
pub async fn wait_message_preserves_retry_budget(shared: RdbCore) {
    //
    test_shared::reset(&shared, WAIT_PREFIX).await;

    let now = OffsetDateTime::now_utc();

    let entry = local_message_entry(
        "rdb-test-prom-wait-page-object",
        "rdb-test-prom-wait-chapter",
        LocalMessageStatus::Processing,
        now,
    );

    let mut conn = shared.get().await.ok().unwrap();

    diesel::insert_into(t_local_message::table)
        .values(&entry)
        .execute(&mut conn)
        .await
        .ok()
        .unwrap();

    diesel::update(
        t_local_message::table
            .filter(t_local_message::f_id.eq("rdb-test-prom-wait-page-object")),
    )
    .set(t_local_message::f_retried_count.eq(3_i64))
    .execute(&mut conn)
    .await
    .ok()
    .unwrap();

    let repo = RdbPromRepo::new();

    let visible_at = now + Duration::minutes(5);

    let mut context =
        RdbContext::<ReptRead>::new(shared.get().await.ok().unwrap());

    repo.step(
        &mut context,
        &RetryMessage::new(
            "rdb-test-prom-wait-page-object",
            Uuid::from_u128(1),
            "page objects are pending",
            &visible_at,
            0,
        ),
    )
    .await
    .ok()
    .unwrap();

    let row: (String, i64) = t_local_message::table
        .filter(t_local_message::f_id.eq("rdb-test-prom-wait-page-object"))
        .select((t_local_message::f_status, t_local_message::f_retried_count))
        .first(&mut conn)
        .await
        .ok()
        .unwrap();

    assert_eq!(row.0, LocalMessageStatus::Pending.as_str());

    assert_eq!(row.1, 3);

    test_shared::cleanup(&shared, WAIT_PREFIX)
        .await
        .ok()
        .unwrap();

    test_shared::assert_no_leftovers(&shared, WAIT_PREFIX)
        .await
        .ok()
        .unwrap();
}

/// Old execution credentials cannot affect a recreated task or a later claim.
pub async fn stale_attempt_finalization_preserves_recreated_task(
    shared: RdbCore,
) {
    //
    test_shared::reset(&shared, LEASE_PREFIX).await;

    let now = OffsetDateTime::now_utc();

    let entry = local_message_entry(
        "rdb-test-prom-claim_token-recreate",
        "rdb-test-prom-claim_token-topic",
        LocalMessageStatus::Pending,
        now,
    );

    let mut conn = shared.get().await.unwrap();

    let repo = RdbPromRepo::new();

    let mut context = RdbContext::<Serial>::new(shared.get().await.unwrap());

    diesel::insert_into(t_local_message::table)
        .values(&entry)
        .execute(&mut conn)
        .await
        .unwrap();

    let old = repo
        .step(&mut context, &ClaimPending::new(1))
        .await
        .unwrap()
        .pop()
        .unwrap();

    repo.step(
        &mut context,
        &CompleteMessage::new(&old.f_id, old.f_claim_token),
    )
    .await
    .unwrap();

    assert_eq!(
        t_local_message::table
            .filter(t_local_message::f_id.eq(&old.f_id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .unwrap(),
        0
    );

    diesel::insert_into(t_local_message::table)
        .values(&entry)
        .execute(&mut conn)
        .await
        .unwrap();

    let current = repo
        .step(&mut context, &ClaimPending::new(1))
        .await
        .unwrap()
        .pop()
        .unwrap();

    assert_ne!(old.f_claim_token, current.f_claim_token);

    // Every old finalization path must leave the recreated attempt unchanged.
    repo.step(
        &mut context,
        &CompleteMessage::new(&old.f_id, old.f_claim_token),
    )
    .await
    .unwrap();

    repo.step(
        &mut context,
        &RetryMessage::new(&old.f_id, old.f_claim_token, "stale", &now, 1),
    )
    .await
    .unwrap();

    repo.step(
        &mut context,
        &FailMessage::new(&old.f_id, old.f_claim_token, "stale"),
    )
    .await
    .unwrap();

    let stored = t_local_message::table
        .filter(t_local_message::f_id.eq(&old.f_id))
        .select((
            t_local_message::f_status,
            t_local_message::f_claim_token,
            t_local_message::f_retried_count,
            t_local_message::f_last_error,
        ))
        .first::<(String, Option<Uuid>, i64, Option<String>)>(&mut conn)
        .await
        .unwrap();

    assert_eq!(
        stored,
        (
            LocalMessageStatus::Processing.as_str().to_owned(),
            Some(current.f_claim_token),
            0,
            None
        )
    );

    repo.step(
        &mut context,
        &ResetStuck::new(&(OffsetDateTime::now_utc() + Duration::seconds(1))),
    )
    .await
    .unwrap();

    repo.step(
        &mut context,
        &CompleteMessage::new(&current.f_id, current.f_claim_token),
    )
    .await
    .unwrap();

    let reclaimed = repo
        .step(&mut context, &ClaimPending::new(1))
        .await
        .unwrap()
        .pop()
        .unwrap();

    assert_ne!(current.f_claim_token, reclaimed.f_claim_token);

    repo.step(
        &mut context,
        &CompleteMessage::new(&current.f_id, current.f_claim_token),
    )
    .await
    .unwrap();

    repo.step(
        &mut context,
        &CompleteMessage::new(&reclaimed.f_id, reclaimed.f_claim_token),
    )
    .await
    .unwrap();

    repo.step(
        &mut context,
        &RetryMessage::new(
            &reclaimed.f_id,
            reclaimed.f_claim_token,
            "late",
            &now,
            1,
        ),
    )
    .await
    .unwrap();

    test_shared::assert_no_leftovers(&shared, LEASE_PREFIX)
        .await
        .unwrap();
}

/// Purges expired dead records while pending and recent dead records remain.
pub async fn dead_message_purge_preserves_pending_records(shared: RdbCore) {
    //
    // Internal state field test_shared.
    test_shared::reset(&shared, PREFIX).await;

    let now = OffsetDateTime::now_utc();

    let pending_entry = LocalMessageEntryRow {
        f_id: "rdb-test-prom-purge-pending",
        f_topic: "image",
        f_status: LocalMessageStatus::Pending,
        f_claim_token: None,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(8),
        f_created_at: now - Duration::days(8),
        f_updated_at: now - Duration::days(8),
    };

    let dead_entry = LocalMessageEntryRow {
        f_id: "rdb-test-prom-purge-dead",
        f_topic: "image",
        f_status: LocalMessageStatus::Dead,
        f_claim_token: None,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(8),
        f_created_at: now - Duration::days(8),
        f_updated_at: now - Duration::days(8),
    };

    let stale_dead_entry = LocalMessageEntryRow {
        f_id: "rdb-test-prom-purge-stale-dead",
        f_topic: "image",
        f_status: LocalMessageStatus::Dead,
        f_claim_token: None,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(31),
        f_created_at: now - Duration::days(31),
        f_updated_at: now - Duration::days(31),
    };

    let mut conn = shared.get().await.ok().unwrap();

    diesel::insert_into(t_local_message::table)
        .values(&[pending_entry, dead_entry, stale_dead_entry])
        .execute(&mut conn)
        .await
        .ok()
        .unwrap();

    let repo = RdbPromRepo::new();

    let dead_before = now - Duration::days(30);

    let mut context =
        RdbContext::<ReptRead>::new(shared.get().await.ok().unwrap());

    let purged_count = repo
        .step(&mut context, &PurgeDead::new(&dead_before))
        .await
        .ok()
        .unwrap();

    assert_eq!(purged_count, 1);

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
        f_claim_token: matches!(status, LocalMessageStatus::Processing)
            .then_some(Uuid::from_u128(1)),
        f_payload: serde_json::json!({}),
        f_visible_at: created_at,
        f_created_at: created_at,
        f_updated_at: created_at,
    }
}
