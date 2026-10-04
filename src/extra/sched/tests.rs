//! Injected task, runner recovery, scheduling, and shutdown regressions.

use std::collections::VecDeque;
use std::future::pending;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::time::{Instant, advance};
use tokio_util::sync::CancellationToken;

use crate::config::sched::SchedTaskConfig;
use crate::result::{BaseError, BaseRest};

use super::task::{SchedNext, SchedTask, SchedTaskRunner};
use super::{Sched, SchedConfig, SchedDesc};

#[derive(Clone, Copy)]
enum Action {
    Next(SchedNext),
    Fail,
    Panic,
    Pending,
    FinishAfterCancel(Duration),
}

#[derive(Default)]
struct Probe {
    configs: AtomicUsize,
    created: AtomicUsize,
    retired: AtomicUsize,
    records: Mutex<Vec<(usize, usize, Instant)>>,

    active: AtomicUsize,
    peak: AtomicUsize,
    exited: AtomicUsize,
    completed: AtomicUsize,

    creation_panics: AtomicBool,
    drop_panics: AtomicBool,
}

struct AttemptGuard(Arc<Probe>);

impl Drop for AttemptGuard {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);

        self.0.exited.fetch_add(1, Ordering::SeqCst);
    }
}

struct TestRunner {
    probe: Arc<Probe>,
    actions: Arc<Mutex<VecDeque<Action>>>,
    id: usize,
    rounds: usize,
}

#[async_trait]
impl SchedTaskRunner for TestRunner {
    async fn run(&mut self, token: CancellationToken) -> BaseRest<SchedNext> {
        self.rounds += 1;

        self.probe.records.lock().unwrap().push((
            self.id,
            self.rounds,
            Instant::now(),
        ));

        let active = self.probe.active.fetch_add(1, Ordering::SeqCst) + 1;

        self.probe.peak.fetch_max(active, Ordering::SeqCst);

        let _guard = AttemptGuard(self.probe.clone());

        let action = self
            .actions
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Action::Next(SchedNext::Wait));

        let result = match action {
            Action::Next(next) => Ok(next),

            Action::Fail => Err(BaseError::Unavailable {
                msg: "injected task failure".into(),
            }),

            Action::Panic => panic!("injected attempt panic"),

            Action::Pending => pending().await,

            Action::FinishAfterCancel(delay) => {
                token.cancelled().await;

                tokio::time::sleep(delay).await;

                Ok(SchedNext::Continue)
            }
        };

        if result.is_ok() {
            self.probe.completed.fetch_add(1, Ordering::SeqCst);
        }

        result
    }
}

impl Drop for TestRunner {
    fn drop(&mut self) {
        self.probe.retired.fetch_add(1, Ordering::SeqCst);

        if self.probe.drop_panics.swap(false, Ordering::SeqCst) {
            panic!("injected runner drop panic");
        }
    }
}

struct TestTask {
    config: SchedTaskConfig,
    probe: Arc<Probe>,
    actions: Arc<Mutex<VecDeque<Action>>>,
}

impl SchedTask for TestTask {
    fn config(&self) -> SchedTaskConfig {
        self.probe.configs.fetch_add(1, Ordering::SeqCst);

        self.config
    }

    fn runner(&self) -> Box<dyn SchedTaskRunner + Send> {
        let id = self.probe.created.fetch_add(1, Ordering::SeqCst);

        if self.probe.creation_panics.swap(false, Ordering::SeqCst) {
            panic!("injected runner creation panic");
        }

        Box::new(TestRunner {
            probe: self.probe.clone(),
            actions: self.actions.clone(),
            id,
            rounds: 0,
        })
    }
}

struct OtherTask(TestTask);

impl SchedTask for OtherTask {
    fn config(&self) -> SchedTaskConfig {
        self.0.config()
    }

    fn runner(&self) -> Box<dyn SchedTaskRunner + Send> {
        self.0.runner()
    }
}

fn task(actions: Vec<Action>) -> (TestTask, Arc<Probe>) {
    let probe = Arc::new(Probe::default());

    let task = TestTask {
        config: SchedTaskConfig::default(),
        probe: probe.clone(),
        actions: Arc::new(Mutex::new(actions.into())),
    };

    (task, probe)
}

fn start(tasks: Vec<Box<dyn SchedTask + Send + Sync>>) -> SchedDesc {
    Sched::new(tasks, SchedConfig::default()).run_detached()
}

async fn settle() {
    for _ in 0..64 {
        tokio::task::yield_now().await;
    }
}

// defaults_bound_attempts_and_shutdown(config)(positive): default scheduling and shutdown intervals match the public contract.
#[test]
fn defaults_bound_attempts_and_shutdown() {
    let config = SchedTaskConfig::default();

    assert_eq!(config.concurrency.get(), 1);

    assert_eq!(config.timeout, Duration::from_secs(300));

    assert_eq!(config.idle_interval, Duration::from_secs(5));

    assert_eq!(config.retry_interval, Duration::from_secs(5));

    assert_eq!(
        SchedConfig::default().shutdown_grace,
        Duration::from_secs(30)
    );
}

// construction_and_early_cancel_do_not_run_tasks(new, cancel)(positive): construction is inert and pre-start cancellation prevents runner creation.
#[tokio::test(start_paused = true)]
async fn construction_and_early_cancel_do_not_run_tasks() {
    let (task, probe) = task(vec![]);

    let sched = Sched::new(vec![Box::new(task)], SchedConfig::default());

    settle().await;

    assert_eq!(probe.configs.load(Ordering::SeqCst), 0);

    assert_eq!(probe.created.load(Ordering::SeqCst), 0);

    let desc = sched.run_detached();

    desc.cancel();

    desc.join().await.unwrap();

    assert_eq!(probe.created.load(Ordering::SeqCst), 0);

    let desc = start(vec![]);

    desc.cancel();

    desc.join().await.unwrap();
}

// heterogeneous_tasks_own_independent_concurrency(run_detached)(positive): each registration gets its own fixed worker count and runner state.
#[tokio::test(start_paused = true)]
async fn heterogeneous_tasks_own_independent_concurrency() {
    let (mut first, first_probe) = task(vec![Action::Pending; 2]);

    let (mut second, second_probe) = task(vec![Action::Pending; 3]);

    first.config.concurrency = NonZeroUsize::new(2).unwrap();

    second.config.concurrency = NonZeroUsize::new(3).unwrap();

    second.config.name = "other_task";

    let desc = start(vec![Box::new(first), Box::new(OtherTask(second))]);

    settle().await;

    for (probe, count) in [(&first_probe, 2), (&second_probe, 3)] {
        assert_eq!(probe.active.load(Ordering::SeqCst), count);

        assert_eq!(probe.created.load(Ordering::SeqCst), count);

        assert_eq!(probe.configs.load(Ordering::SeqCst), 1);

        assert!(
            probe
                .records
                .lock()
                .unwrap()
                .iter()
                .all(|(_, round, _)| *round == 1)
        );
    }

    advance(Duration::from_secs(10)).await;

    settle().await;

    assert_eq!(first_probe.created.load(Ordering::SeqCst), 2);

    assert_eq!(second_probe.created.load(Ordering::SeqCst), 3);

    desc.cancel();

    desc.join().await.unwrap();

    assert_eq!(first_probe.active.load(Ordering::SeqCst), 0);

    assert_eq!(second_probe.active.load(Ordering::SeqCst), 0);
}

// continue_is_immediate_and_wait_uses_idle_interval(run)(positive): backlog draining adds no timer delay while idle rounds respect task policy.
#[tokio::test(start_paused = true)]
async fn continue_is_immediate_and_wait_uses_idle_interval() {
    let (mut task, probe) = task(vec![
        Action::Next(SchedNext::Continue),
        Action::Next(SchedNext::Wait),
    ]);

    task.config.idle_interval = Duration::from_secs(7);

    let desc = start(vec![Box::new(task)]);

    settle().await;

    let records = probe.records.lock().unwrap().clone();

    assert_eq!(records.len(), 2);

    assert_eq!(records[0].2, records[1].2);

    assert_eq!(
        (records[0].0, records[0].1, records[1].0, records[1].1),
        (0, 1, 0, 2)
    );

    advance(Duration::from_secs(6)).await;

    settle().await;

    assert_eq!(probe.records.lock().unwrap().len(), 2);

    advance(Duration::from_secs(1)).await;

    settle().await;

    assert_eq!(probe.records.lock().unwrap().len(), 3);

    desc.cancel();

    desc.join().await.unwrap();
}

// errors_and_panics_retry_without_losing_workers(run)(negative): ordinary errors retain state while runner and worker failures rebuild after the retry interval.
#[tokio::test(start_paused = true)]
async fn errors_and_panics_retry_without_losing_workers() {
    for scenario in 0..4 {
        let action = match scenario {
            0 => Action::Fail,

            1 | 3 => Action::Panic,

            _ => Action::Next(SchedNext::Wait),
        };

        let (mut task, probe) = task(vec![action]);

        task.config.retry_interval = Duration::from_secs(3);

        task.config.idle_interval = Duration::from_secs(60);

        probe.creation_panics.store(scenario == 2, Ordering::SeqCst);

        probe.drop_panics.store(scenario == 3, Ordering::SeqCst);

        let desc = start(vec![Box::new(task)]);

        settle().await;

        let before = probe.records.lock().unwrap().len();

        advance(Duration::from_secs(2)).await;

        settle().await;

        assert_eq!(probe.records.lock().unwrap().len(), before);

        advance(Duration::from_secs(1)).await;

        settle().await;

        let records = probe.records.lock().unwrap().clone();

        assert_eq!(records.len(), before + 1);

        let expected_created = match scenario {
            0 => 1,

            _ => 2,
        };

        assert_eq!(probe.created.load(Ordering::SeqCst), expected_created);

        assert_eq!(probe.configs.load(Ordering::SeqCst), 1);

        assert_eq!(
            records.last().unwrap().1,
            match scenario {
                0 => 2,
                _ => 1,
            }
        );

        desc.cancel();

        desc.join().await.unwrap();
    }
}

// timeout_drops_the_attempt_before_recreating_runner(run)(negative): abandoned attempts release their state before a replacement starts, without overlap.
#[tokio::test(start_paused = true)]
async fn timeout_drops_the_attempt_before_recreating_runner() {
    let (mut task, probe) = task(vec![Action::Pending]);

    task.config.timeout = Duration::from_secs(10);

    task.config.retry_interval = Duration::from_secs(3);

    let desc = start(vec![Box::new(task)]);

    settle().await;

    assert_eq!(probe.active.load(Ordering::SeqCst), 1);

    advance(Duration::from_secs(10)).await;

    settle().await;

    assert_eq!(probe.active.load(Ordering::SeqCst), 0);

    assert_eq!(probe.exited.load(Ordering::SeqCst), 1);

    assert_eq!(probe.completed.load(Ordering::SeqCst), 0);

    assert_eq!(probe.retired.load(Ordering::SeqCst), 1);

    advance(Duration::from_secs(3)).await;

    settle().await;

    assert_eq!(probe.created.load(Ordering::SeqCst), 2);

    assert_eq!(probe.completed.load(Ordering::SeqCst), 1);

    assert_eq!(probe.peak.load(Ordering::SeqCst), 1);

    assert_eq!(probe.records.lock().unwrap().last().unwrap().1, 1);

    desc.cancel();

    desc.join().await.unwrap();
}

// shutdown_uses_one_deadline_for_all_workers(join)(negative): stalled workers are aborted together rather than extending shutdown per worker.
#[tokio::test(start_paused = true)]
async fn shutdown_uses_one_deadline_for_all_workers() {
    let (mut task, probe) = task(vec![Action::Pending; 3]);

    task.config.concurrency = NonZeroUsize::new(3).unwrap();

    let desc = start(vec![Box::new(task)]);

    settle().await;

    let started = Instant::now();

    desc.cancel();

    let joined = tokio::spawn(desc.join());

    settle().await;

    advance(Duration::from_secs(29)).await;

    settle().await;

    assert!(!joined.is_finished());

    assert_eq!(probe.active.load(Ordering::SeqCst), 3);

    advance(Duration::from_secs(1)).await;

    settle().await;

    joined.await.unwrap().unwrap();

    assert!(started.elapsed() <= Duration::from_secs(31));

    assert_eq!(probe.active.load(Ordering::SeqCst), 0);

    assert_eq!(probe.retired.load(Ordering::SeqCst), 3);

    assert_eq!(probe.created.load(Ordering::SeqCst), 3);
}

// shutdown_finishes_current_work_without_another_round(join)(positive): cancellation drains the in-flight operation and prevents new attempts.
#[tokio::test(start_paused = true)]
async fn shutdown_finishes_current_work_without_another_round() {
    let (task, probe) =
        task(vec![Action::FinishAfterCancel(Duration::from_secs(20))]);

    let desc = start(vec![Box::new(task)]);

    settle().await;

    desc.cancel();

    let joined = tokio::spawn(desc.join());

    settle().await;

    assert_eq!(probe.active.load(Ordering::SeqCst), 1);

    advance(Duration::from_secs(20)).await;

    settle().await;

    joined.await.unwrap().unwrap();

    assert_eq!(probe.completed.load(Ordering::SeqCst), 1);

    assert_eq!(probe.records.lock().unwrap().len(), 1);

    assert_eq!(probe.retired.load(Ordering::SeqCst), 1);
}

// idle_wait_and_descriptor_drop_are_cancellable(cancel, drop)(positive): idle workers stop promptly and dropping the owner cancels its group.
#[tokio::test(start_paused = true)]
async fn idle_wait_and_descriptor_drop_are_cancellable() {
    let (mut task, probe) = task(vec![]);

    task.config.idle_interval = Duration::from_secs(3600);

    let desc = start(vec![Box::new(task)]);

    settle().await;

    let started = Instant::now();

    desc.cancel();

    desc.join().await.unwrap();

    assert_eq!(started.elapsed(), Duration::ZERO);

    let (task, drop_probe) = self::task(vec![]);

    let desc = start(vec![Box::new(task)]);

    let token = desc.token.clone();

    settle().await;

    drop(desc);

    settle().await;

    assert!(token.is_cancelled());

    assert_eq!(drop_probe.retired.load(Ordering::SeqCst), 1);

    assert_eq!(probe.records.lock().unwrap().len(), 1);
}

// supervisor_join_errors_are_reported(join)(negative): supervisor failure reaches the owner and its worker group is cancelled.
#[tokio::test(start_paused = true)]
async fn supervisor_join_errors_are_reported() {
    let (task, probe) = task(vec![Action::Pending]);

    let desc = start(vec![Box::new(task)]);

    settle().await;

    desc.task.abort();

    assert!(desc.join().await.unwrap_err().is_cancelled());

    settle().await;

    assert_eq!(probe.active.load(Ordering::SeqCst), 0);
}
