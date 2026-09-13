use super::*;

use crate::part::nucl::{ReptRead, Serial};

use crate::part_impl::prom::rdb_impl::actor::base::RdbPromActor;
use crate::part_impl::prom::rdb_impl::repo::RdbPromRepo;

use crate::shared::test_rdb::start;

#[tokio::test]
#[serial_test::serial(prom_rdb)]
async fn prom_rdb_impls_use_testcontainer() {
    //
    let test_rdb = start().await;

    let shared = test_rdb.core();

    repo::tests::claim_pending_selects_one_visible_message_per_idle_topic(
        shared.clone(),
    )
    .await;

    repo::tests::retry_message_allows_later_topic_message_to_advance(
        shared.clone(),
    )
    .await;

    repo::tests::wait_message_preserves_retry_budget(shared.clone()).await;

    repo::tests::stale_attempt_finalization_preserves_newer_lease(
        shared.clone(),
    )
    .await;

    repo::tests::completed_message_purge_preserves_non_completed_records(
        shared.clone(),
    )
    .await;

    atomic_claim_fences_concurrent_attempts_and_preserves_retry_delay(
        shared.clone(),
    )
    .await;

    competing_snapshots_cannot_process_the_same_topic(shared.clone()).await;

    stale_snapshot_cannot_reclaim_a_delayed_attempt(shared.clone()).await;

    writer_and_consumer_lifecycles_are_independent(shared).await;

    drop(test_rdb);
}

// writer_and_consumer_lifecycles_are_independent(RdbProm/RdbPromActor)(positive): writing is transactional and consumption begins only after explicit startup.
async fn writer_and_consumer_lifecycles_are_independent(
    shared: poprako_rdb_core::RdbCore,
) {
    use crate::part::prom::payload::invitation::InvitationPayload;
    use crate::part_impl::nucl::rdb_impl::RdbNucl;
    use crate::part_impl::obj_dept::tests::ArtworkTestPool;
    use crate::part_impl::obj_dept::{NormObjDept, RdbObjDeptProm};
    use crate::part_impl::repo::HybRepo;
    use crate::part_impl::repo::mock_impl::Mock;
    use diesel::{
        ExpressionMethods as _, QueryDsl as _, TextExpressionMethods as _,
    };
    use poprako_orchestra::{Nucl as _, OperStep as _};

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let writer = RdbProm::new();

    let payload = TaskPayload::Invitation {
        payload: InvitationPayload::PurgeExpiredMemberInvitation {
            invitation_id: "nonexistent".into(),
        },
    };

    let committed_id = "rdb-test-prom-writer-commit".to_string();

    let rollback_id = "rdb-test-prom-writer-rollback".to_string();

    let committed = Task {
        id: &committed_id,
        payload: &payload,
        delay: None,
    };

    nucl.coord(async |context| {
        Defer::new(committed).step_on(&writer, context).await
    })
    .await
    .unwrap();

    let rolled_back = Task {
        id: &rollback_id,
        payload: &payload,
        delay: None,
    };

    let result = nucl
        .coord(async |context| {
            Defer::new(rolled_back).step_on(&writer, context).await?;

            Err::<(), _>(BaseError::Unrecoverable {
                message: "deliberate rollback".into(),
            })
        })
        .await;

    assert!(result.is_err());

    let mut conn = shared.get().await.unwrap();

    let ids = t_local_message::table
        .filter(t_local_message::f_id.like("rdb-test-prom-writer-%"))
        .select(t_local_message::f_id)
        .load::<String>(&mut conn)
        .await
        .unwrap();

    assert_eq!(ids, [committed_id.clone()]);

    let repo = HybRepo::new(shared.clone());

    let dept = NormObjDept::new(
        shared.clone(),
        ArtworkTestPool,
        RdbObjDeptProm::new(shared.clone()),
    );

    let actor = RdbPromActor::new(
        (RdbNucl::<Serial>::new(shared.clone()), RdbPromRepo::new()),
        (nucl.clone(), repo.clone(), dept.view(), Mock::new()),
    );

    tokio::task::yield_now().await;

    let status = t_local_message::table
        .filter(t_local_message::f_id.eq(&committed_id))
        .select(t_local_message::f_status)
        .first::<String>(&mut conn)
        .await
        .unwrap();

    assert_eq!(status, "local_message_status:pending");

    let actor = actor.run_detach();

    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let status = t_local_message::table
                .filter(t_local_message::f_id.eq(&committed_id))
                .select(t_local_message::f_status)
                .first::<String>(&mut conn)
                .await
                .unwrap();

            if status == "local_message_status:completed" {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();

    actor.cancel();

    actor.cancel();

    tokio::time::timeout(std::time::Duration::from_secs(10), actor.join())
        .await
        .unwrap()
        .unwrap();

    let scheduler =
        crate::extra::sched::Sched::new(nucl, repo, dept).run_detach();

    scheduler.cancel();

    tokio::time::timeout(std::time::Duration::from_secs(10), scheduler.join())
        .await
        .unwrap()
        .unwrap();
}

// atomic_claim_fences_concurrent_attempts_and_preserves_retry_delay(ClaimPending)(positive): locked work is skipped, retry visibility is honored, and each attempt gets fresh counters and a new lease.
async fn atomic_claim_fences_concurrent_attempts_and_preserves_retry_delay(
    shared: poprako_rdb_core::RdbCore,
) {
    use crate::part_impl::nucl::rdb_impl::RdbNucl;
    use crate::part_impl::prom::rdb_impl::entity::LocalMessageStatus;
    use crate::part_impl::prom::rdb_impl::repo::{
        ClaimPending, CompleteMessage, ResetStuck, RetryMessage,
    };
    use diesel::{ExpressionMethods as _, QueryDsl as _};
    use poprako_orchestra::{Nucl as _, OperStep as _};
    use time::Duration;

    let prefix = "rdb-test-prom-atomic-";

    test_shared::reset(&shared, prefix).await;

    let mut conn = shared.get().await.unwrap();

    let now = OffsetDateTime::now_utc();

    let entries = ["rdb-test-prom-atomic-first", "rdb-test-prom-atomic-next"]
        .map(|id| LocalMessageEntryRow {
            f_id: id,
            f_topic: "rdb-test-prom-atomic-topic",
            f_status: LocalMessageStatus::Pending,
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

    let (nucl, repo) =
        (RdbNucl::<Serial>::new(shared.clone()), RdbPromRepo::new());

    let mut rows = nucl
        .coord(async |context| {
            let rows = ClaimPending::new(4).step_on(&repo, context).await?;

            let competing = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                nucl.coord(async |context| {
                    ClaimPending::new(4).step_on(&repo, context).await
                }),
            )
            .await
            .unwrap()
            .unwrap();

            assert!(competing.is_empty());

            Ok::<_, BaseError>(rows)
        })
        .await
        .unwrap();

    let first = rows.pop().unwrap();

    assert_eq!(first.f_id, "rdb-test-prom-atomic-first");

    assert_eq!(first.f_lease, 1);

    let later = now + Duration::minutes(5);

    nucl.coord(async |context| {
        RetryMessage::new(&first.f_id, first.f_lease, "retry", &later, 1)
            .step_on(&repo, context)
            .await
    })
    .await
    .unwrap();

    let next = nucl
        .coord(async |context| {
            ClaimPending::new(4).step_on(&repo, context).await
        })
        .await
        .unwrap()
        .pop()
        .unwrap();

    assert_eq!(next.f_id, "rdb-test-prom-atomic-next");

    nucl.coord(async |context| {
        CompleteMessage::new(&next.f_id, next.f_lease)
            .step_on(&repo, context)
            .await
    })
    .await
    .unwrap();

    let rows = nucl
        .coord(async |context| {
            ClaimPending::new(4).step_on(&repo, context).await
        })
        .await
        .unwrap();

    assert!(rows.is_empty());

    diesel::update(
        t_local_message::table.filter(t_local_message::f_id.eq(&first.f_id)),
    )
    .set(t_local_message::f_visible_at.eq(now))
    .execute(&mut conn)
    .await
    .unwrap();

    // Waiting advances leases without consuming the failure budget.
    for expected_lease in 2..6 {
        let attempt = nucl
            .coord(async |context| {
                ClaimPending::new(4).step_on(&repo, context).await
            })
            .await
            .unwrap()
            .pop()
            .unwrap();

        assert_eq!(attempt.f_lease, expected_lease);

        assert_eq!(attempt.f_retried_count, 1);

        nucl.coord(async |context| {
            RetryMessage::new(&attempt.f_id, attempt.f_lease, "wait", &now, 0)
                .step_on(&repo, context)
                .await
        })
        .await
        .unwrap();
    }

    // Recovery consumes the shared failure budget, independently of lease age.
    for expected_retries in 2..=4 {
        let attempt = nucl
            .coord(async |context| {
                ClaimPending::new(4).step_on(&repo, context).await
            })
            .await
            .unwrap()
            .pop()
            .unwrap();

        let cutoff = OffsetDateTime::now_utc() - Duration::minutes(15);

        diesel::update(
            t_local_message::table
                .filter(t_local_message::f_id.eq(&attempt.f_id)),
        )
        .set(t_local_message::f_updated_at.eq(cutoff))
        .execute(&mut conn)
        .await
        .unwrap();

        nucl.coord(async |context| {
            ResetStuck::new(&cutoff).step_on(&repo, context).await
        })
        .await
        .unwrap();

        nucl.coord(async |context| {
            CompleteMessage::new(&attempt.f_id, attempt.f_lease)
                .step_on(&repo, context)
                .await
        })
        .await
        .unwrap();

        let (status, retried_count) = t_local_message::table
            .filter(t_local_message::f_id.eq(&attempt.f_id))
            .select((
                t_local_message::f_status,
                t_local_message::f_retried_count,
            ))
            .first::<(String, i64)>(&mut conn)
            .await
            .unwrap();

        let expected_status = match expected_retries {
            4 => LocalMessageStatus::Dead,
            _ => LocalMessageStatus::Pending,
        };

        assert_eq!(status, expected_status.as_str());

        assert_eq!(retried_count, expected_retries.min(3));
    }

    test_shared::cleanup(&shared, prefix).await.unwrap();
}

// competing_snapshots_cannot_process_the_same_topic(ClaimPending)(negative): a serializable snapshot that selects a different record cannot violate topic exclusivity.
async fn competing_snapshots_cannot_process_the_same_topic(
    shared: poprako_rdb_core::RdbCore,
) {
    use crate::part_impl::nucl::rdb_impl::RdbNucl;
    use crate::part_impl::prom::rdb_impl::entity::LocalMessageStatus;
    use crate::part_impl::prom::rdb_impl::repo::{
        ClaimPending, CompleteMessage,
    };
    use diesel::{ExpressionMethods as _, QueryDsl as _};
    use poprako_orchestra::{Nucl as _, OperStep as _};
    use time::Duration;

    let prefix = "rdb-test-prom-snapshot-";

    test_shared::reset(&shared, prefix).await;

    let now = OffsetDateTime::now_utc();

    let mut entry = LocalMessageEntryRow {
        f_id: "rdb-test-prom-snapshot-next",
        f_topic: "rdb-test-prom-snapshot-topic",
        f_status: LocalMessageStatus::Pending,
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
                    ClaimPending::new(4).step_on(&repo, context).await
                })
                .await
                .unwrap();

            assert_eq!(rows[0].f_id, entry.f_id);

            ClaimPending::new(4).step_on(&repo, context).await
        })
        .await;

    assert!(matches!(
        result.map_err(BaseError::from),
        Err(BaseError::Retryable { .. })
    ));

    let processing_count = t_local_message::table
        .filter(t_local_message::f_topic.eq(entry.f_topic))
        .filter(
            t_local_message::f_status
                .eq(LocalMessageStatus::Processing.as_str()),
        )
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    assert_eq!(processing_count, 1);

    nucl.coord(async |context| {
        CompleteMessage::new(entry.f_id, 1)
            .step_on(&repo, context)
            .await
    })
    .await
    .unwrap();

    let rows = nucl
        .coord(async |context| {
            ClaimPending::new(4).step_on(&repo, context).await
        })
        .await
        .unwrap();

    assert_eq!(rows[0].f_id, "rdb-test-prom-snapshot-next");

    test_shared::cleanup(&shared, prefix).await.unwrap();
}

// stale_snapshot_cannot_reclaim_a_delayed_attempt(ClaimPending)(negative): a snapshot predating another attempt cannot bypass the committed Wait visibility deadline.
async fn stale_snapshot_cannot_reclaim_a_delayed_attempt(
    shared: poprako_rdb_core::RdbCore,
) {
    use crate::part_impl::nucl::rdb_impl::RdbNucl;
    use crate::part_impl::prom::rdb_impl::entity::LocalMessageStatus;
    use crate::part_impl::prom::rdb_impl::repo::{ClaimPending, RetryMessage};
    use diesel::QueryDsl as _;
    use poprako_orchestra::{Nucl as _, OperStep as _};
    use time::Duration;

    let prefix = "rdb-test-prom-stale-";

    test_shared::reset(&shared, prefix).await;

    let now = OffsetDateTime::now_utc();

    let entry = LocalMessageEntryRow {
        f_id: "rdb-test-prom-stale-attempt",
        f_topic: "rdb-test-prom-stale-topic",
        f_status: LocalMessageStatus::Pending,
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
                    ClaimPending::new(4).step_on(&repo, context).await
                })
                .await
                .unwrap()
                .pop()
                .unwrap();

            let later = now + Duration::minutes(5);

            nucl.coord(async |context| {
                RetryMessage::new(
                    &attempt.f_id,
                    attempt.f_lease,
                    "wait",
                    &later,
                    0,
                )
                .step_on(&repo, context)
                .await
            })
            .await
            .unwrap();

            ClaimPending::new(4).step_on(&repo, context).await
        })
        .await;

    assert!(result.is_err());

    let rows = nucl
        .coord(async |context| {
            ClaimPending::new(4).step_on(&repo, context).await
        })
        .await
        .unwrap();

    assert!(rows.is_empty());

    test_shared::cleanup(&shared, prefix).await.unwrap();
}
