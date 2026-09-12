use diesel::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Nucl as _, OperStep as _};
use poprako_rdb_core::RdbCore;
use time::{Duration, OffsetDateTime};

use crate::part::nucl::Serial;
use crate::part::prom::oper::Defer;
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part::prom::task::Task;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::prom::rdb_impl::RdbProm;
use crate::part_impl::prom::rdb_impl::entity::LocalMessageRow;
use crate::part_impl::prom::rdb_impl::repo::{
    ClaimPending, CompleteMessage, RdbPromRepo, ResetStuck, RetryMessage,
};
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::result::BaseError;

use super::pool::enforce_retry_limit;
use super::task_flow::TaskFlow;

#[test]
fn fourth_failure_becomes_dead() {
    let task_flow = enforce_retry_limit(
        TaskFlow::Retry {
            err_message: "failed".into(),
        },
        3,
    );

    assert!(matches!(task_flow, TaskFlow::Dead { .. }));
}

#[test]
fn first_three_failures_remain_retryable() {
    for retried_count in 0..3 {
        let task_flow = enforce_retry_limit(
            TaskFlow::Retry {
                err_message: "failed".into(),
            },
            retried_count,
        );

        assert!(matches!(task_flow, TaskFlow::Retry { .. }));
    }
}

#[test]
fn waiting_does_not_consume_retry_limit() {
    let task_flow = enforce_retry_limit(
        TaskFlow::Wait {
            err_message: "external state is pending".into(),
        },
        i64::MAX,
    );

    assert!(matches!(task_flow, TaskFlow::Wait { .. }));
}

// dispatch_uses_injected_repo(RdbPromActor::dispatch_payload)(positive): a decoded task mutates the injected mock without accessing queue storage.
#[tokio::test]
async fn dispatch_uses_injected_repo() {
    use crate::model::read::proj::member_invitation::MemberInvitationInfo;
    use crate::part::prom::payload::TaskPayload;
    use crate::part::prom::payload::invitation::InvitationPayload;
    use crate::part_impl::nucl::rdb_impl::RdbNucl;
    use crate::part_impl::prom::rdb_impl::actor::base::RdbPromActor;
    use crate::part_impl::prom::rdb_impl::repo::RdbPromRepo;
    use crate::part_impl::repo::mock_impl::Mock;
    use crate::value::role::{RoleField, RoleMask};
    use poprako_rdb_core::RdbCore;

    let core = RdbCore::from_database_url(
        "postgres://unused:unused@127.0.0.1:1/unused",
    )
    .unwrap();

    let mock = Mock::new();

    mock.seed_member_invitation(MemberInvitationInfo {
        id: "invitation".into(),
        team_id: "team".into(),
        invitor: None,
        invitor_id: "owner".into(),
        invitee_qid: "qid".into(),
        code: "code".into(),
        is_pending: true,
        roles: RoleMask::from(RoleField::TRANSLATOR),
    });

    let actor = RdbPromActor::new(
        (RdbNucl::new(core), RdbPromRepo::new()),
        (mock.clone(), mock.clone(), mock.clone(), mock.clone()),
    );

    let payload = TaskPayload::Invitation {
        payload: InvitationPayload::Member {
            invitation_id: "invitation".into(),
        },
    };

    tokio::task::yield_now().await;

    assert_eq!(mock.snapshot().member_invitations.len(), 1);

    let rejected = actor
        .dispatch_payload(
            "invalid_topic",
            &serde_json::to_value(&payload).unwrap(),
        )
        .await;

    assert!(matches!(rejected, TaskFlow::Dead { .. }));

    assert_eq!(mock.snapshot().member_invitations.len(), 1);

    let flow = actor
        .dispatch_payload(
            payload.topic(),
            &serde_json::to_value(&payload).unwrap(),
        )
        .await;

    assert!(matches!(flow, TaskFlow::Complete));

    assert!(mock.snapshot().member_invitations.is_empty());
}

// Builds a production chapter-check payload with identifiable request ownership.
fn chapter_payload(chapter_id: &str, actor_user_id: &str) -> TaskPayload {
    TaskPayload::Chapter {
        payload: ChapterPayload::TryAdvanceRawProvideStage {
            chapter_id: chapter_id.into(),
            actor_user_id: actor_user_id.into(),
        },
    }
}

// Defers through the production writer inside its caller-owned transaction.
async fn write_chapter_task(
    nucl: &RdbNucl<Serial>,
    id: &str,
    chapter_id: &str,
    actor_user_id: &str,
    delay: u64,
) {
    let id = id.to_owned();

    let payload = chapter_payload(chapter_id, actor_user_id);

    let task = Task {
        id: &id,
        payload: &payload,
        delay: Some(std::time::Duration::from_secs(delay)),
    };

    nucl.coord(async |context| {
        Defer::new(task).step_on(&RdbProm::new(), context).await
    })
    .await
    .unwrap();
}

// Claims committed attempts using the production queue repository.
async fn claim_tasks(
    nucl: &RdbNucl<Serial>,
    limit: usize,
) -> Vec<LocalMessageRow> {
    nucl.coord(async |context| {
        ClaimPending::new(limit)
            .step_on(&RdbPromRepo::new(), context)
            .await
    })
    .await
    .unwrap()
}

// Acknowledges only the supplied attempt lease.
async fn complete_task(nucl: &RdbNucl<Serial>, row: &LocalMessageRow) {
    nucl.coord(async |context| {
        CompleteMessage::new(&row.f_id, row.f_lease)
            .step_on(&RdbPromRepo::new(), context)
            .await
    })
    .await
    .unwrap();
}

// Captures persisted task state for lifecycle assertions.
async fn persisted_task(
    core: &RdbCore,
    id: &str,
) -> (
    String,
    serde_json::Value,
    OffsetDateTime,
    OffsetDateTime,
    i64,
    i64,
) {
    let mut conn = core.get().await.unwrap();

    t_local_message::table
        .filter(t_local_message::f_id.eq(id))
        .select((
            t_local_message::f_status,
            t_local_message::f_payload,
            t_local_message::f_created_at,
            t_local_message::f_visible_at,
            t_local_message::f_retried_count,
            t_local_message::f_lease,
        ))
        .first(&mut conn)
        .await
        .unwrap()
}

// task_categories_define_concurrency(ClaimPending)(positive): different chapters share one topic while invitations can run alongside them.
async fn task_categories_define_concurrency(core: &RdbCore) {
    use crate::part::prom::payload::invitation::InvitationPayload;

    let nucl = RdbNucl::<Serial>::new(core.clone());

    for index in 0..2 {
        write_chapter_task(
            &nucl,
            &format!("category-task-{index}"),
            &format!("chapter-{index}"),
            "actor",
            0,
        )
        .await;
    }

    let id = "category-invitation".to_owned();

    let payload = TaskPayload::Invitation {
        payload: InvitationPayload::Member {
            invitation_id: "invitation".into(),
        },
    };

    let task = Task {
        id: &id,
        payload: &payload,
        delay: None,
    };

    nucl.coord(async |context| {
        Defer::new(task).step_on(&RdbProm::new(), context).await
    })
    .await
    .unwrap();

    assert!(claim_tasks(&nucl, 0).await.is_empty());

    let first = claim_tasks(&nucl, 1).await;

    assert_eq!(first.len(), 1);

    assert_eq!(first[0].f_topic, "advance_raw_provide");

    let second = claim_tasks(&nucl, 4).await;

    assert_eq!(second.len(), 1);

    assert_eq!(second[0].f_topic, "purge_expired_invitation");

    assert!(claim_tasks(&nucl, 4).await.is_empty());

    complete_task(&nucl, &first[0]).await;

    let next = claim_tasks(&nucl, 4).await;

    assert_eq!(next.len(), 1);

    assert_eq!(next[0].f_id, "category-task-1");

    complete_task(&nucl, &next[0]).await;

    complete_task(&nucl, &second[0]).await;
}

// same_topic_requests_remain_independent(Defer/RetryMessage/ResetStuck)(positive): later tasks never replace or complete an earlier attempt.
async fn same_topic_requests_remain_independent(core: &RdbCore) {
    let nucl = RdbNucl::<Serial>::new(core.clone());

    for action in ["wait", "retry", "timeout"] {
        let first_id = format!("independent-{action}-first");

        let second_id = format!("independent-{action}-second");

        write_chapter_task(&nucl, &first_id, action, "first", 0).await;

        let first = claim_tasks(&nucl, 4).await.remove(0);

        write_chapter_task(&nucl, &second_id, action, "second", 0).await;

        let second = persisted_task(core, &second_id).await;

        assert!(claim_tasks(&nucl, 4).await.is_empty());

        nucl.coord(async |context| match action {
            "timeout" => {
                ResetStuck::new(
                    &(OffsetDateTime::now_utc() + Duration::seconds(1)),
                )
                .step_on(&RdbPromRepo::new(), context)
                .await
            }

            _ => {
                RetryMessage::new(
                    &first.f_id,
                    first.f_lease,
                    action,
                    &OffsetDateTime::now_utc(),
                    i64::from(action == "retry"),
                )
                .step_on(&RdbPromRepo::new(), context)
                .await
            }
        })
        .await
        .unwrap();

        let pending = persisted_task(core, &first_id).await;

        assert_eq!(pending.0, "local_message_status:pending");

        assert_eq!(pending.4, i64::from(action != "wait"));

        assert_eq!(persisted_task(core, &second_id).await, second);

        let attempts = claim_tasks(&nucl, 4).await;

        assert_eq!(attempts.len(), 1);

        assert_eq!(attempts[0].f_id, first_id);

        complete_task(&nucl, &first).await;

        assert_eq!(
            persisted_task(core, &first_id).await.0,
            "local_message_status:processing"
        );

        complete_task(&nucl, &attempts[0]).await;

        let attempts = claim_tasks(&nucl, 4).await;

        assert_eq!(attempts.len(), 1);

        assert_eq!(attempts[0].f_id, second_id);

        complete_task(&nucl, &attempts[0]).await;
    }
}

// same_topic_batch_is_transactional(DeferBatch)(positive): all requests survive a committed batch and none survive rollback.
async fn same_topic_batch_is_transactional(core: &RdbCore) {
    use crate::part::prom::oper::DeferBatch;

    let nucl = RdbNucl::<Serial>::new(core.clone());

    let ids = ["batch-first".to_owned(), "batch-second".to_owned()];

    let payload = chapter_payload("batch-chapter", "actor");

    let tasks = ids
        .iter()
        .map(|id| Task {
            id,
            payload: &payload,
            delay: None,
        })
        .collect::<Vec<_>>();

    let rolled_back = nucl
        .coord(async |context| {
            DeferBatch::new(&tasks)
                .step_on(&RdbProm::new(), context)
                .await?;

            Err::<(), _>(BaseError::Unrecoverable {
                message: "test rollback".into(),
            })
        })
        .await;

    assert!(rolled_back.is_err());

    assert!(claim_tasks(&nucl, 4).await.is_empty());

    nucl.coord(async |context| {
        DeferBatch::new(&tasks)
            .step_on(&RdbProm::new(), context)
            .await
    })
    .await
    .unwrap();

    for id in ids {
        let rows = claim_tasks(&nucl, 4).await;

        assert_eq!(rows.len(), 1);

        assert_eq!(rows[0].f_id, id);

        assert!(claim_tasks(&nucl, 4).await.is_empty());

        complete_task(&nucl, &rows[0]).await;
    }
}

// topic_claim_and_independent_requests_use_testcontainer(RdbProm)(positive): topic concurrency and independent task lifecycle hold through the production writer.
#[tokio::test]
#[serial_test::serial(prom_rdb)]
async fn topic_claim_and_independent_requests_use_testcontainer() {
    let test_rdb = crate::shared::test_rdb::start().await;

    let core = test_rdb.core();

    task_categories_define_concurrency(&core).await;

    same_topic_requests_remain_independent(&core).await;

    same_topic_batch_is_transactional(&core).await;
}
