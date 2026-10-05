#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// dead_message_purge_preserves_pending_records(PurgeDead)(positive): expired dead records are purged while pending and recent dead records remain.
// claim_pending_selects_one_visible_message_per_idle_topic(ClaimPending)(positive): each claim reads its requested topic and skips processing work in that topic.
// retry_message_allows_later_topic_message_to_advance(RetryTask)(positive): delayed retries are equivalent to re-enqueueing behind visible work.
// wait_message_preserves_retry_budget(RetryTask)(positive): waiting for external state returns the task to Pending without incrementing its retry counter.
// stale_attempt_finalization_preserves_recreated_task(CompleteTask/RetryTask/FailTask)(negative): an expired worker claim_token cannot finalize a newer processing attempt or overwrite Dead.

// Verifies topic isolation for claiming, recovery, and retention.
mod topic_isolation;

use super::repo::*;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Nucl as _, OperStep as _, Step as _};
use time::Duration;
use time::OffsetDateTime;
use uuid::Uuid;

use poprako_prom::general::delivery::ClaimedTask;
use poprako_prom::general::rdb_impl::LocalTaskStatus;
use poprako_rdb_core::{RdbConn, RdbCore};

use crate::part::nucl::{ReptRead, Serial};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::prom::rdb_impl::entity::LocalTaskEntryRow;
use crate::part_impl::prom::rdb_impl::test_shared;
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::shared::RdbContext;

// Constant definition for `PREFIX`.
const PREFIX: &str = "rdb-test-prom-purge-";
// Constant definition for `POLL_PREFIX`.
const POLL_PREFIX: &str = "rdb-test-prom-poll-";
// Constant definition for `LEASE_PREFIX`.
const LEASE_PREFIX: &str = "rdb-test-prom-claim_token-";
// Constant definition for `WAIT_PREFIX`.
const WAIT_PREFIX: &str = "rdb-test-prom-wait-";

// Verify complete, retry, and dead-letter actions from an expired credential change nothing.
async fn verify_stale_finalization(
    repo: &RdbPromRepo,
    context: &mut RdbContext<Serial>,
    conn: &mut RdbConn,
    (old, current): (&ClaimedTask, &ClaimedTask),
    now: OffsetDateTime,
) {
    // Every old finalization path must leave the recreated attempt unchanged.
    repo.step(context, &CompleteTask::new(old.id(), old.claim_token()))
        .await
        .unwrap();

    repo.step(
        context,
        &RetryTask::new(old.id(), old.claim_token(), "stale", &now, 1),
    )
    .await
    .unwrap();

    repo.step(
        context,
        &FailTask::new(old.id(), old.claim_token(), "stale"),
    )
    .await
    .unwrap();

    let stored = t_local_message::table
        .filter(t_local_message::f_id.eq(old.id()))
        .select((
            t_local_message::f_status,
            t_local_message::f_claim_token,
            t_local_message::f_retried_count,
            t_local_message::f_last_error,
        ))
        .first::<(String, Option<Uuid>, i64, Option<String>)>(conn)
        .await
        .unwrap();

    assert_eq!(
        stored,
        (
            LocalTaskStatus::Processing.as_str().to_owned(),
            Some(current.claim_token()),
            0,
            None
        )
    );
}

/// Verifies each topic claims its own oldest visible message independently.
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
            LocalTaskStatus::Pending,
            now - Duration::minutes(5),
        ),
        local_message_entry(
            "rdb-test-prom-poll-image-second",
            "rdb-test-prom-poll-image",
            LocalTaskStatus::Pending,
            now - Duration::minutes(4),
        ),
        local_message_entry(
            "rdb-test-prom-poll-invitation",
            "rdb-test-prom-poll-invitation",
            LocalTaskStatus::Pending,
            now - Duration::minutes(3),
        ),
        local_message_entry(
            "rdb-test-prom-poll-chapter-processing",
            "rdb-test-prom-poll-chapter",
            LocalTaskStatus::Processing,
            now - Duration::minutes(2),
        ),
        local_message_entry(
            "rdb-test-prom-poll-chapter-pending",
            "rdb-test-prom-poll-chapter",
            LocalTaskStatus::Pending,
            now - Duration::minutes(1),
        ),
    ];

    let mut conn = shared.get().await.unwrap();

    diesel::insert_into(t_local_message::table)
        .values(&entries)
        .execute(&mut conn)
        .await
        .unwrap();

    let repo = RdbPromRepo::new();

    let nucl = RdbNucl::<Serial>::new(shared.clone());

    let image = nucl
        .coord(async |context| {
            ClaimPending::new("rdb-test-prom-poll-image")
                .step_on(&repo, context)
                .await
        })
        .await
        .unwrap()
        .unwrap();

    assert_eq!(image.id(), "rdb-test-prom-poll-image-first");

    let invitation = nucl
        .coord(async |context| {
            ClaimPending::new("rdb-test-prom-poll-invitation")
                .step_on(&repo, context)
                .await
        })
        .await
        .unwrap()
        .unwrap();

    assert_eq!(invitation.id(), "rdb-test-prom-poll-invitation");

    let chapter = nucl
        .coord(async |context| {
            ClaimPending::new("rdb-test-prom-poll-chapter")
                .step_on(&repo, context)
                .await
        })
        .await
        .unwrap();

    assert!(chapter.is_none());

    test_shared::cleanup(&shared, POLL_PREFIX).await.unwrap();

    test_shared::assert_no_leftovers(&shared, POLL_PREFIX)
        .await
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
            LocalTaskStatus::Processing,
            now - Duration::minutes(2),
        ),
        local_message_entry(
            "rdb-test-prom-poll-image-next",
            "rdb-test-prom-poll-retry-image",
            LocalTaskStatus::Pending,
            now - Duration::minutes(1),
        ),
    ];

    let mut conn = shared.get().await.unwrap();

    diesel::insert_into(t_local_message::table)
        .values(&entries)
        .execute(&mut conn)
        .await
        .unwrap();

    let repo = RdbPromRepo::new();

    let retry_visible_at = now + Duration::minutes(5);

    let mut context = RdbContext::<ReptRead>::new(shared.get().await.unwrap());

    repo.step(
        &mut context,
        &RetryTask::new(
            "rdb-test-prom-poll-image-retry",
            Uuid::from_u128(1),
            "temporary failure",
            &retry_visible_at,
            1,
        ),
    )
    .await
    .unwrap();

    let row = RdbNucl::<Serial>::new(shared.clone())
        .coord(async |context| {
            ClaimPending::new("rdb-test-prom-poll-retry-image")
                .step_on(&repo, context)
                .await
        })
        .await
        .unwrap()
        .unwrap();

    assert_eq!(row.id(), "rdb-test-prom-poll-image-next");

    test_shared::cleanup(&shared, POLL_PREFIX).await.unwrap();

    test_shared::assert_no_leftovers(&shared, POLL_PREFIX)
        .await
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
        LocalTaskStatus::Processing,
        now,
    );

    let mut conn = shared.get().await.unwrap();

    diesel::insert_into(t_local_message::table)
        .values(&entry)
        .execute(&mut conn)
        .await
        .unwrap();

    diesel::update(
        t_local_message::table
            .filter(t_local_message::f_id.eq("rdb-test-prom-wait-page-object")),
    )
    .set(t_local_message::f_retried_count.eq(3_i64))
    .execute(&mut conn)
    .await
    .unwrap();

    let repo = RdbPromRepo::new();

    let visible_at = now + Duration::minutes(5);

    let mut context = RdbContext::<ReptRead>::new(shared.get().await.unwrap());

    repo.step(
        &mut context,
        &RetryTask::new(
            "rdb-test-prom-wait-page-object",
            Uuid::from_u128(1),
            "page objects are pending",
            &visible_at,
            0,
        ),
    )
    .await
    .unwrap();

    let row: (String, i64) = t_local_message::table
        .filter(t_local_message::f_id.eq("rdb-test-prom-wait-page-object"))
        .select((t_local_message::f_status, t_local_message::f_retried_count))
        .first(&mut conn)
        .await
        .unwrap();

    assert_eq!(row.0, LocalTaskStatus::Pending.as_str());

    assert_eq!(row.1, 3);

    test_shared::cleanup(&shared, WAIT_PREFIX).await.unwrap();

    test_shared::assert_no_leftovers(&shared, WAIT_PREFIX)
        .await
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
        LocalTaskStatus::Pending,
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
        .step(&mut context, &ClaimPending::new(entry.f_topic))
        .await
        .unwrap()
        .unwrap();

    repo.step(
        &mut context,
        &CompleteTask::new(old.id(), old.claim_token()),
    )
    .await
    .unwrap();

    assert_eq!(
        t_local_message::table
            .filter(t_local_message::f_id.eq(old.id()))
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
        .step(&mut context, &ClaimPending::new(entry.f_topic))
        .await
        .unwrap()
        .unwrap();

    assert_ne!(old.claim_token(), current.claim_token());

    verify_stale_finalization(
        &repo,
        &mut context,
        &mut conn,
        (&old, &current),
        now,
    )
    .await;

    verify_reclaimed_attempt(&repo, &mut context, &current, entry.f_topic, now)
        .await;

    test_shared::assert_no_leftovers(&shared, LEASE_PREFIX)
        .await
        .unwrap();
}

/// Purges expired dead records while pending and recent dead records remain.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub async fn dead_message_purge_preserves_pending_records(shared: RdbCore) {
    //
    // Internal state field test_shared.
    test_shared::reset(&shared, PREFIX).await;

    let now = OffsetDateTime::now_utc();

    let pending_entry = LocalTaskEntryRow {
        f_id: "rdb-test-prom-purge-pending",
        f_topic: "image",
        f_status: LocalTaskStatus::Pending,
        f_claim_token: None,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(8),
        f_created_at: now - Duration::days(8),
        f_updated_at: now - Duration::days(8),
    };

    let dead_entry = LocalTaskEntryRow {
        f_id: "rdb-test-prom-purge-dead",
        f_topic: "image",
        f_status: LocalTaskStatus::Dead,
        f_claim_token: None,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(8),
        f_created_at: now - Duration::days(8),
        f_updated_at: now - Duration::days(8),
    };

    let stale_dead_entry = LocalTaskEntryRow {
        f_id: "rdb-test-prom-purge-stale-dead",
        f_topic: "image",
        f_status: LocalTaskStatus::Dead,
        f_claim_token: None,
        f_payload: serde_json::json!({}),
        f_visible_at: now - Duration::days(31),
        f_created_at: now - Duration::days(31),
        f_updated_at: now - Duration::days(31),
    };

    let mut conn = shared.get().await.unwrap();

    diesel::insert_into(t_local_message::table)
        .values(&[pending_entry, dead_entry, stale_dead_entry])
        .execute(&mut conn)
        .await
        .unwrap();

    let repo = RdbPromRepo::new();

    let dead_before = now - Duration::days(30);

    let mut context = RdbContext::<ReptRead>::new(shared.get().await.unwrap());

    let purged_count = repo
        .step(&mut context, &PurgeDead::new("image", &dead_before))
        .await
        .unwrap();

    assert_eq!(purged_count, 1);

    let remaining_ids: Vec<String> = t_local_message::table
        .filter(t_local_message::f_id.like(format!("{}%", PREFIX)))
        .order_by(t_local_message::f_id.asc())
        .select(t_local_message::f_id)
        .load(&mut conn)
        .await
        .unwrap();

    assert_eq!(
        remaining_ids,
        vec![
            "rdb-test-prom-purge-dead".to_string(),
            "rdb-test-prom-purge-pending".to_string(),
        ]
    );

    test_shared::cleanup(&shared, PREFIX).await.unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .unwrap();
}

// Internal implementation of `local_message_entry`.
fn local_message_entry(
    id: &'static str,
    topic: &'static str,
    status: LocalTaskStatus,
    created_at: OffsetDateTime,
) -> LocalTaskEntryRow<'static> {
    LocalTaskEntryRow {
        f_id: id,
        f_topic: topic,
        f_status: status,
        f_claim_token: matches!(status, LocalTaskStatus::Processing)
            .then_some(Uuid::from_u128(1)),
        f_payload: serde_json::json!({}),
        f_visible_at: created_at,
        f_created_at: created_at,
        f_updated_at: created_at,
    }
}

async fn verify_reclaimed_attempt(
    repo: &RdbPromRepo,
    context: &mut RdbContext<Serial>,
    current: &ClaimedTask,
    topic: &str,
    now: OffsetDateTime,
) {
    repo.step(
        context,
        &ResetStuck::new(
            topic,
            &(OffsetDateTime::now_utc() + Duration::seconds(1)),
        ),
    )
    .await
    .unwrap();

    repo.step(
        context,
        &CompleteTask::new(current.id(), current.claim_token()),
    )
    .await
    .unwrap();

    let reclaimed = repo
        .step(context, &ClaimPending::new(topic))
        .await
        .unwrap()
        .unwrap();

    assert_ne!(current.claim_token(), reclaimed.claim_token());

    repo.step(
        context,
        &CompleteTask::new(current.id(), current.claim_token()),
    )
    .await
    .unwrap();

    repo.step(
        context,
        &CompleteTask::new(reclaimed.id(), reclaimed.claim_token()),
    )
    .await
    .unwrap();

    repo.step(
        context,
        &RetryTask::new(
            reclaimed.id(),
            reclaimed.claim_token(),
            "late",
            &now,
            1,
        ),
    )
    .await
    .unwrap();
}
