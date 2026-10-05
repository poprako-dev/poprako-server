#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::sync::{Mutex, Notify};
use uuid::Uuid;

use crate::general::actor::PromActor;
use crate::general::delivery::{ClaimedTask, Delivery};
use crate::general::dispatch_flow::DispatchFlow;
use crate::general::handler::{Dispatcher, Handler, Payload};

#[derive(Serialize, Deserialize)]
enum Work {
    First(u8),
    Second(u8),
}

impl Payload for Work {
    const TOPICS: &'static [&'static str] = &["first", "second"];

    fn topic(&self) -> &'static str {
        match self {
            Self::First(_) => "first",

            Self::Second(_) => "second",
        }
    }
}

struct CountingHandler(Arc<AtomicUsize>);

impl Handler for CountingHandler {
    type Payload = Work;

    async fn handle(&self, _payload: Work) -> DispatchFlow {
        self.0.fetch_add(1, Ordering::SeqCst);

        DispatchFlow::Complete
    }
}

#[derive(Serialize, Deserialize)]
struct OtherWork(String);

impl Payload for OtherWork {
    const TOPICS: &'static [&'static str] = &["other"];

    fn topic(&self) -> &'static str {
        "other"
    }
}

// associated_payload_selects_decoding(Handler)(positive): named and function handlers own unrelated payload contracts.
#[tokio::test]
async fn associated_payload_selects_decoding() {
    let calls = Arc::new(AtomicUsize::new(0));

    let actor = PromActor::new((), CountingHandler(calls.clone()));

    let flow = actor
        .dispatch_payload("first", &serde_json::json!({"First": 1}))
        .await;

    assert!(matches!(flow, DispatchFlow::Complete));

    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let actor = PromActor::new(
        (),
        Dispatcher::new((), |(), payload: OtherWork| async move {
            DispatchFlow::Wait { err_msg: payload.0 }
        }),
    );

    let flow = actor
        .dispatch_payload("other", &serde_json::json!("waiting"))
        .await;

    assert!(
        matches!(flow, DispatchFlow::Wait { err_msg } if err_msg == "waiting")
    );
}

// invalid_envelopes_never_invoke_handler(Actor)(negative): JSON errors and mismatched queues terminate before business work.
#[tokio::test]
async fn invalid_envelopes_never_invoke_handler() {
    let calls = Arc::new(AtomicUsize::new(0));

    let actor = PromActor::new((), CountingHandler(calls.clone()));

    for (topic, payload) in [
        ("first", serde_json::json!("corrupt")),
        ("second", serde_json::json!({"First": 1})),
        ("unknown", serde_json::json!({"First": 1})),
    ] {
        let flow = actor.dispatch_payload(topic, &payload).await;

        assert!(matches!(flow, DispatchFlow::Dead { .. }));
    }

    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

type DeliveryEvent = (String, &'static str, i64);

#[derive(Clone, Default)]
struct MemoryDelivery {
    tasks: Arc<Mutex<VecDeque<ClaimedTask>>>,
    events: Arc<Mutex<Vec<DeliveryEvent>>>,
    changed: Arc<Notify>,
}

impl MemoryDelivery {
    async fn wait_events(&self, count: usize) {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let changed = self.changed.notified();

                if self.events.lock().await.len() >= count {
                    return;
                }

                changed.await;
            }
        })
        .await
        .unwrap();
    }

    async fn record(&self, id: &str, action: &'static str, delta: i64) {
        self.events
            .lock()
            .await
            .push((id.to_owned(), action, delta));

        self.changed.notify_one();
    }
}

impl Delivery for MemoryDelivery {
    type Error = &'static str;

    async fn claim(
        &self,
        topic: &str,
    ) -> Result<Option<ClaimedTask>, Self::Error> {
        let mut tasks = self.tasks.lock().await;

        let task = tasks
            .iter()
            .position(|task| task.topic() == topic)
            .and_then(|index| tasks.remove(index));

        drop(tasks);

        Ok(task)
    }

    async fn complete(&self, task: &ClaimedTask) -> Result<(), Self::Error> {
        self.record(task.id(), "complete", 0).await;

        Ok(())
    }

    async fn retry(
        &self,
        task: &ClaimedTask,
        _message: &str,
        _visible_at: OffsetDateTime,
        retry_delta: i64,
    ) -> Result<(), Self::Error> {
        self.record(task.id(), "retry", retry_delta).await;

        Ok(())
    }

    async fn dead(
        &self,
        task: &ClaimedTask,
        _message: &str,
    ) -> Result<(), Self::Error> {
        self.record(task.id(), "dead", 0).await;

        Ok(())
    }

    async fn reset_stuck(
        &self,
        _topic: &str,
        _before: OffsetDateTime,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn purge_dead(
        &self,
        _topic: &str,
        _before: OffsetDateTime,
    ) -> Result<usize, Self::Error> {
        Ok(0)
    }
}

fn task(id: &str, work: &Work) -> ClaimedTask {
    ClaimedTask::new(
        (
            id.to_owned(),
            work.topic().to_owned(),
            serde_json::to_value(work).unwrap(),
        ),
        (0, Uuid::from_u128(1), OffsetDateTime::now_utc()),
    )
}

// topics_progress_independently(Actor)(positive): a blocked topic reserves capacity while another topic completes.
#[tokio::test]
async fn topics_progress_independently() {
    let delivery = MemoryDelivery::default();

    delivery.tasks.lock().await.extend([
        task("first-1", &Work::First(1)),
        task("first-2", &Work::First(2)),
        task("second", &Work::Second(0)),
    ]);

    let entered = Arc::new(Notify::new());

    let release = Arc::new(Notify::new());

    let actor = PromActor::new(
        delivery.clone(),
        Dispatcher::new(
            (entered.clone(), release.clone()),
            |(entered, release), work: Work| async move {
                if matches!(work, Work::First(1)) {
                    entered.notify_one();

                    release.notified().await;
                }

                DispatchFlow::Complete
            },
        ),
    )
    .run_detached();

    entered.notified().await;

    delivery.wait_events(1).await;

    assert_eq!(
        *delivery.events.lock().await,
        vec![("second".into(), "complete", 0)]
    );

    assert_eq!(delivery.tasks.lock().await.len(), 1);

    release.notify_one();

    delivery.wait_events(3).await;

    let events = delivery.events.lock().await.clone();

    assert_eq!(
        events,
        vec![
            ("second".into(), "complete", 0),
            ("first-1".into(), "complete", 0),
            ("first-2".into(), "complete", 0),
        ]
    );

    actor.cancel_and_join().await.unwrap();
}

// dispatch_flows_preserve_delivery_policy(Actor)(positive): retry budgets, wait deadlines, completion, and fatal failures reach their delivery operations.
#[tokio::test]
async fn dispatch_flows_preserve_delivery_policy() {
    let delivery = MemoryDelivery::default();

    let exhausted = ClaimedTask::new(
        (
            "exhausted".into(),
            "first".into(),
            serde_json::to_value(Work::First(1)).unwrap(),
        ),
        (3, Uuid::from_u128(1), OffsetDateTime::now_utc()),
    );

    let expired = ClaimedTask::new(
        (
            "expired".into(),
            "first".into(),
            serde_json::to_value(Work::First(2)).unwrap(),
        ),
        (
            0,
            Uuid::from_u128(1),
            OffsetDateTime::now_utc() - time::Duration::hours(2),
        ),
    );

    delivery.tasks.lock().await.extend([
        task("complete", &Work::First(0)),
        task("retry", &Work::First(1)),
        task("wait", &Work::First(2)),
        task("dead", &Work::First(3)),
        exhausted,
        expired,
    ]);

    let actor = PromActor::new(
        delivery.clone(),
        Dispatcher::new((), |(), work: Work| async move {
            match work {
                Work::First(0) => DispatchFlow::Complete,

                Work::First(1) => DispatchFlow::Retry {
                    err_msg: "retry".into(),
                },

                Work::First(2) => DispatchFlow::Wait {
                    err_msg: "wait".into(),
                },

                _ => DispatchFlow::Dead {
                    err_msg: "fatal".into(),
                },
            }
        }),
    )
    .run_detached();

    delivery.wait_events(6).await;

    actor.cancel_and_join().await.unwrap();

    assert_eq!(
        *delivery.events.lock().await,
        vec![
            ("complete".into(), "complete", 0),
            ("retry".into(), "retry", 1),
            ("wait".into(), "retry", 0),
            ("dead".into(), "dead", 0),
            ("exhausted".into(), "dead", 0),
            ("expired".into(), "dead", 0),
        ]
    );
}

// cancel_and_join_drains_started_attempt(ActorDesc)(positive): shutdown stops claiming and waits for the running handler.
#[tokio::test]
async fn cancel_and_join_drains_started_attempt() {
    let delivery = MemoryDelivery::default();

    delivery.tasks.lock().await.extend([
        task("started", &Work::First(1)),
        task("pending", &Work::First(2)),
    ]);

    let release = Arc::new(Notify::new());

    let entered = Arc::new(Notify::new());

    let actor = PromActor::new(
        delivery.clone(),
        Dispatcher::<Work, _, _>::new(
            (release.clone(), entered.clone()),
            |(release, entered), _| async move {
                entered.notify_one();

                release.notified().await;

                DispatchFlow::Complete
            },
        ),
    )
    .run_detached();

    entered.notified().await;

    actor.cancel();

    let joined = tokio::spawn(actor.join());

    tokio::task::yield_now().await;

    assert!(!joined.is_finished());

    release.notify_one();

    tokio::time::timeout(std::time::Duration::from_secs(5), joined)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    assert_eq!(delivery.tasks.lock().await.len(), 1);

    assert_eq!(
        *delivery.events.lock().await,
        vec![("started".into(), "complete", 0)]
    );
}
