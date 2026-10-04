use diesel::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Nucl as _, OperStep as _};
use poprako_orchestra_extra::prom::oper::Defer;
use poprako_orchestra_extra::prom::task::Task;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use poprako_prom::general::delivery::ClaimedTask;
use poprako_prom::general::dispatch_flow::DispatchFlow;
use poprako_rdb_core::RdbCore;

use crate::part::nucl::Serial;
use crate::part::prom::payload::PromPayload;
use crate::part::prom::payload::chapter::ChapterPayload;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::prom::rdb_impl::repo::{
    ClaimPending, CompleteTask, RdbPromRepo, ResetStuck, RetryTask,
};
use crate::part_impl::prom::rdb_impl::writer::RdbProm;
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::result::BaseError;

// dispatch_uses_injected_repo(Actor::dispatch_payload)(positive): a decoded task mutates the injected mock without accessing queue storage.
#[tokio::test]
async fn dispatch_uses_injected_repo() {
    use crate::model::read::proj::member_invitation::MemberInvitationInfo;
    use crate::part::prom::payload::PromPayload;
    use crate::part::prom::payload::invitation::InvitationPayload;
    use crate::part_impl::prom::dispatch;
    use crate::part_impl::repo::mock_impl::Mock;
    use crate::part_impl::repo::mock_impl::MockContext;
    use crate::value::role::{RoleField, RoleMask};
    use poprako_prom::general::actor::PromActor;
    use poprako_prom::general::handler::Dispatcher;

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

    let actor = PromActor::new(
        (),
        Dispatcher::new(mock.clone(), |mock, payload| async move {
            dispatch::dispatch::<MockContext, _, _, _, _>(
                (&mock, &mock, &mock, &mock),
                payload,
            )
            .await
        }),
    );

    let payload = PromPayload::Invitation {
        payload: InvitationPayload::PurgeExpiredMemberInvitation {
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

    assert!(matches!(rejected, DispatchFlow::Dead { .. }));

    assert_eq!(mock.snapshot().member_invitations.len(), 1);

    let flow = actor
        .dispatch_payload(
            payload.topic().as_str(),
            &serde_json::to_value(&payload).unwrap(),
        )
        .await;

    assert!(matches!(flow, DispatchFlow::Complete));

    assert!(mock.snapshot().member_invitations.is_empty());
}

// Builds a production chapter-check payload with identifiable request ownership.
fn chapter_payload(chapter_id: &str, actor_user_id: &str) -> PromPayload {
    PromPayload::Chapter {
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
async fn claim_task(
    nucl: &RdbNucl<Serial>,
    topic: &str,
) -> Option<ClaimedTask> {
    nucl.coord(async |context| {
        ClaimPending::new(topic)
            .step_on(&RdbPromRepo::new(), context)
            .await
    })
    .await
    .unwrap()
}

// Acknowledges only the supplied attempt claim_token.
async fn complete_task(nucl: &RdbNucl<Serial>, row: &ClaimedTask) {
    nucl.coord(async |context| {
        CompleteTask::new(row.id(), row.claim_token())
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
    Option<Uuid>,
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
            t_local_message::f_claim_token,
        ))
        .first(&mut conn)
        .await
        .unwrap()
}

// fixed_topics_control_concurrency(ClaimPending)(positive): different payload kinds share one serial queue while another topic runs concurrently.
async fn fixed_topics_control_concurrency(core: &RdbCore) {
    use crate::part::prom::payload::invitation::InvitationPayload;

    let nucl = RdbNucl::<Serial>::new(core.clone());

    for index in 0..2 {
        write_chapter_task(
            &nucl,
            &format!("category-task-{}", index),
            &format!("chapter-{}", index),
            "actor",
            0,
        )
        .await;
    }

    let invitations = [
        InvitationPayload::PurgeExpiredMemberInvitation {
            invitation_id: "invitation".into(),
        },
        InvitationPayload::PurgeExpiredAssignmentInvitation {
            invitation_id: "invitation".into(),
        },
    ];

    for (index, invitation_payload) in invitations.into_iter().enumerate() {
        let id = format!("category-invitation-{}", index);

        let payload = PromPayload::Invitation {
            payload: invitation_payload,
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
    }

    let first = claim_task(&nucl, "chapter").await.unwrap();

    assert_eq!(first.topic(), "chapter");

    let second = claim_task(&nucl, "invitation").await.unwrap();

    assert_eq!(second.id(), "category-invitation-0");

    assert!(claim_task(&nucl, "chapter").await.is_none());

    assert!(claim_task(&nucl, "invitation").await.is_none());

    complete_task(&nucl, &first).await;

    let next = claim_task(&nucl, "chapter").await.unwrap();

    assert_eq!(next.id(), "category-task-1");

    complete_task(&nucl, &next).await;

    assert!(claim_task(&nucl, "invitation").await.is_none());

    complete_task(&nucl, &second).await;

    let next = claim_task(&nucl, "invitation").await.unwrap();

    assert_eq!(next.id(), "category-invitation-1");

    assert_eq!(next.topic(), "invitation");

    complete_task(&nucl, &next).await;
}

// same_topic_requests_remain_independent(Defer/RetryTask/ResetStuck)(positive): later tasks never replace or complete an earlier attempt.
async fn same_topic_requests_remain_independent(core: &RdbCore) {
    let nucl = RdbNucl::<Serial>::new(core.clone());

    for action in ["wait", "retry", "timeout"] {
        let first_id = format!("independent-{}-first", action);

        let second_id = format!("independent-{}-second", action);

        write_chapter_task(&nucl, &first_id, action, "first", 0).await;

        let first = claim_task(&nucl, "chapter").await.unwrap();

        write_chapter_task(&nucl, &second_id, action, "second", 0).await;

        let second = persisted_task(core, &second_id).await;

        assert!(claim_task(&nucl, "chapter").await.is_none());

        nucl.coord(async |context| match action {
            "timeout" => {
                ResetStuck::new(
                    "chapter",
                    &(OffsetDateTime::now_utc() + Duration::seconds(1)),
                )
                .step_on(&RdbPromRepo::new(), context)
                .await
            }

            _ => {
                RetryTask::new(
                    &first.id(),
                    first.claim_token(),
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

        let attempt = claim_task(&nucl, "chapter").await.unwrap();

        assert_eq!(attempt.id(), first_id);

        complete_task(&nucl, &first).await;

        assert_eq!(
            persisted_task(core, &first_id).await.0,
            "local_message_status:processing"
        );

        complete_task(&nucl, &attempt).await;

        let attempt = claim_task(&nucl, "chapter").await.unwrap();

        assert_eq!(attempt.id(), second_id);

        complete_task(&nucl, &attempt).await;
    }
}

// same_topic_batch_is_transactional(DeferBatch)(positive): all requests survive a committed batch and none survive rollback.
async fn same_topic_batch_is_transactional(core: &RdbCore) {
    use poprako_orchestra_extra::prom::oper::DeferBatch;

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
                msg: "test rollback".into(),
            })
        })
        .await;

    assert!(rolled_back.is_err());

    assert!(claim_task(&nucl, "chapter").await.is_none());

    nucl.coord(async |context| {
        DeferBatch::new(&tasks)
            .step_on(&RdbProm::new(), context)
            .await
    })
    .await
    .unwrap();

    for id in ids {
        let row = claim_task(&nucl, "chapter").await.unwrap();

        assert_eq!(row.id(), id);

        assert!(claim_task(&nucl, "chapter").await.is_none());

        complete_task(&nucl, &row).await;
    }
}

// topic_claim_and_independent_requests_use_testcontainer(RdbProm)(positive): topic concurrency and independent task lifecycle hold through the production writer.
#[tokio::test]
#[serial_test::serial(prom_rdb)]
async fn topic_claim_and_independent_requests_use_testcontainer() {
    let test_rdb = crate::shared::test_rdb::start().await;

    let core = test_rdb.core();

    fixed_topics_control_concurrency(&core).await;

    same_topic_requests_remain_independent(&core).await;

    same_topic_batch_is_transactional(&core).await;
}

// writer_and_consumer_lifecycles_are_independent(RdbProm/Actor)(positive): writing is transactional and consumption begins only after explicit startup.
pub async fn writer_and_consumer_lifecycles_are_independent(
    shared: poprako_rdb_core::RdbCore,
) {
    use crate::part::nucl::ReptRead;
    use crate::part::prom::payload::invitation::InvitationPayload;
    use crate::part_impl::nucl::rdb_impl::RdbNucl;
    use crate::part_impl::obj_dept::tests::ArtworkTestPool;
    use crate::part_impl::obj_dept::{NormObjDept, RdbObjDeptProm};
    use crate::part_impl::prom::dispatch;
    use crate::part_impl::prom::rdb_impl::delivery::RdbPromDelivery;
    use crate::part_impl::repo::HybRepo;
    use crate::part_impl::repo::mock_impl::Mock;
    use crate::{Sched, SchedConfig, SubtreeDeleteTask};
    use diesel::{
        ExpressionMethods as _, QueryDsl as _, TextExpressionMethods as _,
    };
    use poprako_orchestra::{Nucl as _, OperStep as _};
    use poprako_prom::general::actor::PromActor;
    use poprako_prom::general::handler::Dispatcher;

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let writer = RdbProm::new();

    let payload = PromPayload::Invitation {
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
                msg: "deliberate rollback".into(),
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

    let actor = PromActor::new(
        RdbPromDelivery::new(
            RdbNucl::<Serial>::new(shared.clone()),
            RdbPromRepo::new(),
        ),
        Dispatcher::new(
            (nucl.clone(), repo.clone(), dept.view(), Mock::new()),
            |(nucl, repo, view, develop), payload| async move {
                dispatch::dispatch((&nucl, &repo, &view, &develop), payload)
                    .await
            },
        ),
    );

    tokio::task::yield_now().await;

    let status = t_local_message::table
        .filter(t_local_message::f_id.eq(&committed_id))
        .select(t_local_message::f_status)
        .first::<String>(&mut conn)
        .await
        .unwrap();

    assert_eq!(status, "local_message_status:pending");

    let actor = actor.run_detached();

    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let status = t_local_message::table
                .filter(t_local_message::f_id.eq(&committed_id))
                .select(t_local_message::f_status)
                .count()
                .get_result::<i64>(&mut conn)
                .await
                .unwrap();

            if status == 0 {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();

    actor.cancel();

    tokio::time::timeout(std::time::Duration::from_secs(10), actor.join())
        .await
        .unwrap()
        .unwrap();

    let task = Box::new(SubtreeDeleteTask::new(nucl, repo, dept));

    let sched = Sched::new(vec![task], SchedConfig::default()).run_detached();

    sched.cancel();

    tokio::time::timeout(std::time::Duration::from_secs(10), sched.join())
        .await
        .unwrap()
        .unwrap();
}
