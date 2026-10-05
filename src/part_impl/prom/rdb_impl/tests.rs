#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::entity::LocalTaskEntryRow;
use crate::part_impl::prom::rdb_impl::repo::{
    ClaimPending, CompleteTask, ResetStuck,
};
use diesel::{ExpressionMethods as _, QueryDsl as _};
use poprako_orchestra::{Nucl as _, OperStep as _};
use poprako_prom::general::rdb_impl::LocalTaskStatus;
use poprako_rdb_core::{RdbConn, RdbPooledConn};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use super::*;

use crate::part::nucl::Serial;
use crate::part_impl::prom::rdb_impl::repo::RdbPromRepo;
use crate::result::BaseError;
use crate::shared::test_rdb::start;
use diesel_async::RunQueryDsl as _;

// Recover stuck attempts until the shared failure budget dead-letters the task.
async fn verify_shared_retry_budget(
    nucl: &RdbNucl<Serial>,
    repo: &RdbPromRepo,
    conn: &mut RdbConn,
    topic: &str,
) {
    // Recovery consumes the shared failure budget, independently of claim_token age.
    for expected_retries in 2..=4 {
        let attempt = nucl
            .coord(async |context| {
                ClaimPending::new(topic).step_on(repo, context).await
            })
            .await
            .unwrap()
            .unwrap();

        let cutoff = OffsetDateTime::now_utc() - Duration::minutes(15);

        diesel::update(
            t_local_message::table
                .filter(t_local_message::f_id.eq(attempt.id())),
        )
        .set(t_local_message::f_updated_at.eq(cutoff))
        .execute(conn)
        .await
        .unwrap();

        nucl.coord(async |context| {
            ResetStuck::new(topic, &cutoff).step_on(repo, context).await
        })
        .await
        .unwrap();

        nucl.coord(async |context| {
            CompleteTask::new(attempt.id(), attempt.claim_token())
                .step_on(repo, context)
                .await
        })
        .await
        .unwrap();

        let (status, retried_count) = t_local_message::table
            .filter(t_local_message::f_id.eq(attempt.id()))
            .select((
                t_local_message::f_status,
                t_local_message::f_retried_count,
            ))
            .first::<(String, i64)>(conn)
            .await
            .unwrap();

        let expected_status = match expected_retries {
            4 => LocalTaskStatus::Dead,
            _ => LocalTaskStatus::Pending,
        };

        assert_eq!(status, expected_status.as_str());

        assert_eq!(retried_count, expected_retries.min(3));
    }
}

#[tokio::test]
#[serial_test::serial(prom_rdb)]
async fn prom_rdb_impls_use_testcontainer() {
    //
    let test_rdb = start().await;

    let shared = test_rdb.core();

    repo_tests::claim_pending_selects_one_visible_message_per_idle_topic(
        shared.clone(),
    )
    .await;

    repo_tests::retry_message_allows_later_topic_message_to_advance(
        shared.clone(),
    )
    .await;

    repo_tests::wait_message_preserves_retry_budget(shared.clone()).await;

    repo_tests::stale_attempt_finalization_preserves_recreated_task(
        shared.clone(),
    )
    .await;

    repo_tests::dead_message_purge_preserves_pending_records(shared.clone())
        .await;

    atomic_claim_fences_concurrent_attempts_and_preserves_retry_delay(
        shared.clone(),
    )
    .await;

    competing_snapshots_cannot_process_the_same_topic(shared.clone()).await;

    stale_snapshot_cannot_reclaim_a_delayed_attempt(shared.clone()).await;

    actor_tests::writer_and_consumer_lifecycles_are_independent(shared).await;

    drop(test_rdb);
}

// atomic_claim_fences_concurrent_attempts_and_preserves_retry_delay(ClaimPending)(positive): locked work is skipped, retry visibility is honored, and each attempt gets fresh counters and a new claim_token.
async fn atomic_claim_fences_concurrent_attempts_and_preserves_retry_delay(
    shared: poprako_rdb_core::RdbCore,
) {
    use crate::part_impl::nucl::rdb_impl::RdbNucl;
    use crate::part_impl::prom::rdb_impl::repo::{
        ClaimPending, CompleteTask, RetryTask,
    };
    use diesel::{ExpressionMethods as _, QueryDsl as _};
    use poprako_orchestra::{Nucl as _, OperStep as _};
    use time::Duration;

    let topic = "rdb-test-prom-atomic-topic";

    let prefix = "rdb-test-prom-atomic-";

    test_shared::reset(&shared, prefix).await;

    let (mut conn, now) = seed_atomic_tasks(&shared, topic).await;

    let (nucl, repo) =
        (RdbNucl::<Serial>::new(shared.clone()), RdbPromRepo::new());

    let first = nucl
        .coord(async |context| {
            let rows = ClaimPending::new(topic).step_on(&repo, context).await?;

            let competing = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                nucl.coord(async |context| {
                    ClaimPending::new(topic).step_on(&repo, context).await
                }),
            )
            .await
            .unwrap()
            .unwrap();

            assert!(competing.is_none());

            Ok::<_, BaseError>(rows)
        })
        .await
        .unwrap()
        .unwrap();

    assert_eq!(first.id(), "rdb-test-prom-atomic-first");

    assert!(!first.claim_token().is_nil());

    let later = now + Duration::minutes(5);

    nucl.coord(async |context| {
        RetryTask::new(first.id(), first.claim_token(), "retry", &later, 1)
            .step_on(&repo, context)
            .await
    })
    .await
    .unwrap();

    let next = nucl
        .coord(async |context| {
            ClaimPending::new(topic).step_on(&repo, context).await
        })
        .await
        .unwrap()
        .unwrap();

    assert_eq!(next.id(), "rdb-test-prom-atomic-next");

    nucl.coord(async |context| {
        CompleteTask::new(next.id(), next.claim_token())
            .step_on(&repo, context)
            .await
    })
    .await
    .unwrap();

    let rows = nucl
        .coord(async |context| {
            ClaimPending::new(topic).step_on(&repo, context).await
        })
        .await
        .unwrap();

    assert!(rows.is_none());

    diesel::update(
        t_local_message::table.filter(t_local_message::f_id.eq(first.id())),
    )
    .set(t_local_message::f_visible_at.eq(now))
    .execute(&mut conn)
    .await
    .unwrap();

    // Waiting rotates execution credentials without consuming the failure budget.
    let mut tokens = std::collections::HashSet::from([first.claim_token()]);

    for _ in 0..4 {
        let attempt = nucl
            .coord(async |context| {
                ClaimPending::new(topic).step_on(&repo, context).await
            })
            .await
            .unwrap()
            .unwrap();

        assert!(tokens.insert(attempt.claim_token()));

        assert_eq!(attempt.retried_count(), 1);

        nucl.coord(async |context| {
            RetryTask::new(attempt.id(), attempt.claim_token(), "wait", &now, 0)
                .step_on(&repo, context)
                .await
        })
        .await
        .unwrap();
    }

    verify_shared_retry_budget(&nucl, &repo, &mut conn, topic).await;

    test_shared::cleanup(&shared, prefix).await.unwrap();
}

// competing_snapshots_cannot_process_the_same_topic(ClaimPending)(negative): a serializable snapshot that selects a different record cannot violate topic exclusivity.
async fn competing_snapshots_cannot_process_the_same_topic(
    shared: poprako_rdb_core::RdbCore,
) {
    use crate::part_impl::nucl::rdb_impl::RdbNucl;
    use crate::part_impl::prom::rdb_impl::repo::{ClaimPending, CompleteTask};
    use diesel::{ExpressionMethods as _, QueryDsl as _};
    use poprako_orchestra::{Nucl as _, OperStep as _};
    use poprako_prom::general::rdb_impl::LocalTaskStatus;
    use time::Duration;

    let topic = "rdb-test-prom-snapshot-topic";

    let prefix = "rdb-test-prom-snapshot-";

    test_shared::reset(&shared, prefix).await;

    let now = OffsetDateTime::now_utc();

    let mut entry = LocalTaskEntryRow {
        f_id: "rdb-test-prom-snapshot-next",
        f_topic: topic,
        f_status: LocalTaskStatus::Pending,
        f_claim_token: None,
        f_payload: serde_json::json!({}),
        f_visible_at: now,
        f_created_at: now,
        f_updated_at: now,
    };

    let mut conn = shared.get().await.unwrap();

    diesel::insert_into(t_local_message::table)
        .values(&entry)
        .execute(&mut conn)
        .await
        .unwrap();

    let (nucl, repo) =
        (RdbNucl::<Serial>::new(shared.clone()), RdbPromRepo::new());

    let result = nucl
        .coord(async |context| {
            // Establish the older snapshot before inserting a new oldest task.
            let count = t_local_message::table
                .filter(t_local_message::f_topic.eq(entry.f_topic))
                .count()
                .get_result::<i64>(context.conn())
                .await
                .unwrap();

            assert!(count > 0);

            entry.f_id = "rdb-test-prom-snapshot-first";

            entry.f_created_at = now - Duration::minutes(1);

            diesel::insert_into(t_local_message::table)
                .values(&entry)
                .execute(&mut conn)
                .await
                .unwrap();

            let rows = nucl
                .coord(async |context| {
                    ClaimPending::new(topic).step_on(&repo, context).await
                })
                .await
                .unwrap();

            assert_eq!(rows.unwrap().id(), entry.f_id);

            ClaimPending::new(topic).step_on(&repo, context).await
        })
        .await;

    assert!(matches!(
        result.map_err(BaseError::from),
        Err(BaseError::Retryable { .. })
    ));

    let processing_tokens = t_local_message::table
        .filter(t_local_message::f_topic.eq(entry.f_topic))
        .filter(
            t_local_message::f_status.eq(LocalTaskStatus::Processing.as_str()),
        )
        .select(t_local_message::f_claim_token)
        .load::<Option<Uuid>>(&mut conn)
        .await
        .unwrap();

    assert_eq!(processing_tokens.len(), 1);

    nucl.coord(async |context| {
        CompleteTask::new(
            entry.f_id,
            processing_tokens.as_slice().first().unwrap().unwrap(),
        )
        .step_on(&repo, context)
        .await
    })
    .await
    .unwrap();

    let rows = nucl
        .coord(async |context| {
            ClaimPending::new(topic).step_on(&repo, context).await
        })
        .await
        .unwrap();

    assert_eq!(rows.unwrap().id(), "rdb-test-prom-snapshot-next");

    test_shared::cleanup(&shared, prefix).await.unwrap();
}

// stale_snapshot_cannot_reclaim_a_delayed_attempt(ClaimPending)(negative): a snapshot predating another attempt cannot bypass the committed Wait visibility deadline.
async fn stale_snapshot_cannot_reclaim_a_delayed_attempt(
    shared: poprako_rdb_core::RdbCore,
) {
    use crate::part_impl::nucl::rdb_impl::RdbNucl;
    use crate::part_impl::prom::rdb_impl::repo::{ClaimPending, RetryTask};
    use diesel::QueryDsl as _;
    use poprako_orchestra::{Nucl as _, OperStep as _};
    use poprako_prom::general::rdb_impl::LocalTaskStatus;
    use time::Duration;

    let topic = "rdb-test-prom-stale-topic";

    let prefix = "rdb-test-prom-stale-";

    test_shared::reset(&shared, prefix).await;

    let now = OffsetDateTime::now_utc();

    let entry = LocalTaskEntryRow {
        f_id: "rdb-test-prom-stale-attempt",
        f_topic: topic,
        f_status: LocalTaskStatus::Pending,
        f_claim_token: None,
        f_payload: serde_json::json!({}),
        f_visible_at: now,
        f_created_at: now,
        f_updated_at: now,
    };

    let mut conn = shared.get().await.unwrap();

    diesel::insert_into(t_local_message::table)
        .values(&entry)
        .execute(&mut conn)
        .await
        .unwrap();

    let (nucl, repo) =
        (RdbNucl::<Serial>::new(shared.clone()), RdbPromRepo::new());

    let result = nucl
        .coord(async |context| {
            t_local_message::table
                .count()
                .get_result::<i64>(context.conn())
                .await
                .unwrap();

            let attempt = nucl
                .coord(async |context| {
                    ClaimPending::new(topic).step_on(&repo, context).await
                })
                .await
                .unwrap()
                .unwrap();

            let later = now + Duration::minutes(5);

            nucl.coord(async |context| {
                RetryTask::new(
                    attempt.id(),
                    attempt.claim_token(),
                    "wait",
                    &later,
                    0,
                )
                .step_on(&repo, context)
                .await
            })
            .await
            .unwrap();

            ClaimPending::new(topic).step_on(&repo, context).await
        })
        .await;

    assert!(result.is_err());

    let rows = nucl
        .coord(async |context| {
            ClaimPending::new(topic).step_on(&repo, context).await
        })
        .await
        .unwrap();

    assert!(rows.is_none());

    test_shared::cleanup(&shared, prefix).await.unwrap();
}

async fn seed_atomic_tasks(
    shared: &poprako_rdb_core::RdbCore,
    topic: &'static str,
) -> (RdbPooledConn, OffsetDateTime) {
    let mut conn = shared.get().await.unwrap();

    let now = OffsetDateTime::now_utc();

    let entries = ["rdb-test-prom-atomic-first", "rdb-test-prom-atomic-next"]
        .map(|id| LocalTaskEntryRow {
            f_id: id,
            f_topic: topic,
            f_status: LocalTaskStatus::Pending,
            f_claim_token: None,
            f_payload: serde_json::json!({}),
            f_visible_at: now,
            f_created_at: now,
            f_updated_at: now,
        });

    diesel::insert_into(t_local_message::table)
        .values(&entries)
        .execute(&mut conn)
        .await
        .unwrap();

    (conn, now)
}
