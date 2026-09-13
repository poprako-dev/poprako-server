use super::*;

use tokio::sync::{Semaphore, mpsc};

// Supplies isolated execution guards to lifecycle tests with queued attempts.
fn permit() -> OwnedSemaphorePermit {
    Arc::new(Semaphore::new(1)).try_acquire_owned().unwrap()
}

// timed_out_attempt_releases_worker(run_worker)(negative): dropping a pending attempt frees its worker for the next task.
#[tokio::test]
async fn timed_out_attempt_releases_worker() {
    let (work_send, work_recv) = mpsc::unbounded_channel();

    let cancelled = tokio_util::sync::CancellationToken::new();

    let finished = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let deadline = Instant::now() + StdDuration::from_millis(100);

    work_send.send((true, deadline, permit())).unwrap();

    work_send
        .send((false, Instant::now() + StdDuration::from_secs(1), permit()))
        .unwrap();

    drop(work_send);

    let run = run_worker(work_recv, Arc::new(Notify::new()), |hang| {
        let cancelled = cancelled.clone();

        let finished = finished.clone();

        async move {
            if hang {
                let _drop_guard = cancelled.drop_guard();

                std::future::pending::<()>().await;
            }

            finished.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    });

    timeout(StdDuration::from_secs(1), run).await.unwrap();

    assert!(cancelled.is_cancelled());

    assert!(finished.load(std::sync::atomic::Ordering::SeqCst));
}

// panicked_attempt_releases_worker(run_worker)(negative): a task panic leaves the worker available to subsequent tasks.
#[tokio::test]
async fn panicked_attempt_releases_worker() {
    let (work_send, work_recv) = mpsc::unbounded_channel();

    let finished = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let deadline = Instant::now() + StdDuration::from_secs(1);

    work_send.send((true, deadline, permit())).unwrap();

    work_send.send((false, deadline, permit())).unwrap();

    drop(work_send);

    let run = run_worker(work_recv, Arc::new(Notify::new()), |should_panic| {
        let finished = finished.clone();

        async move {
            assert!(!should_panic, "injected task panic");

            finished.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    });

    timeout(StdDuration::from_secs(1), run).await.unwrap();

    assert!(finished.load(std::sync::atomic::Ordering::SeqCst));
}

// expired_queued_attempt_is_not_started(run_worker)(negative): a row that exhausted its deadline in the queue must not invoke business code.
#[tokio::test]
async fn expired_queued_attempt_is_not_started() {
    let (work_send, work_recv) = mpsc::unbounded_channel();

    work_send.send(((), Instant::now(), permit())).unwrap();

    drop(work_send);

    let started = std::sync::atomic::AtomicBool::new(false);

    run_worker(work_recv, Arc::new(Notify::new()), |()| async {
        started.store(true, std::sync::atomic::Ordering::SeqCst);
    })
    .await;

    assert!(!started.load(std::sync::atomic::Ordering::SeqCst));
}

// shutdown_aborts_pending_workers(shutdown_workers)(negative): the shared grace period cancels pending attempts instead of hanging on drain.
#[tokio::test]
async fn shutdown_aborts_pending_workers() {
    let mut workers = JoinSet::new();

    let cancelled = tokio_util::sync::CancellationToken::new();

    let (started_send, started_recv) = tokio::sync::oneshot::channel();

    let drop_guard = cancelled.clone().drop_guard();

    workers.spawn(async move {
        let _drop_guard = drop_guard;

        started_send.send(()).unwrap();

        std::future::pending::<()>().await;
    });

    started_recv.await.unwrap();

    timeout(
        StdDuration::from_secs(1),
        shutdown_workers(workers, StdDuration::from_millis(20)),
    )
    .await
    .unwrap();

    timeout(StdDuration::from_secs(1), cancelled.cancelled())
        .await
        .unwrap();
}

// workers_reserve_capacity_until_completion(WorkerSlot/run_worker)(positive): all four workers execute together while consumed queue items still reserve capacity.
#[tokio::test]
async fn workers_reserve_capacity_until_completion() {
    let completed = Arc::new(Notify::new());

    let release = Arc::new(Semaphore::new(0));

    let (started_send, mut started_recv) = mpsc::unbounded_channel();

    let mut slots = Vec::new();

    let mut workers = JoinSet::new();

    for task_id in 0..WORKER_COUNT {
        let (slot, work_recv) = WorkerSlot::channel();

        let started_send = started_send.clone();

        let release = release.clone();

        workers.spawn(run_worker(
            work_recv,
            completed.clone(),
            move |task_id| {
                let started_send = started_send.clone();

                let release = release.clone();

                async move {
                    started_send.send(task_id).unwrap();

                    release.acquire().await.unwrap().forget();
                }
            },
        ));

        let permit = slot.acquire().unwrap();

        assert!(slot.acquire().is_none());

        assert!(slot.dispatch(
            task_id,
            Instant::now() + StdDuration::from_secs(10),
            permit
        ));

        slots.push(slot);
    }

    let mut started_ids = Vec::new();

    for _ in 0..WORKER_COUNT {
        let task_id = timeout(StdDuration::from_secs(1), started_recv.recv())
            .await
            .unwrap()
            .unwrap();

        started_ids.push(task_id);
    }

    started_ids.sort_unstable();

    assert_eq!(started_ids, (0..WORKER_COUNT).collect::<Vec<_>>());

    assert!(slots.iter().all(|slot| slot.acquire().is_none()));

    release.add_permits(1);

    timeout(StdDuration::from_secs(1), completed.notified())
        .await
        .unwrap();

    let reservations = slots
        .iter()
        .filter_map(WorkerSlot::acquire)
        .collect::<Vec<_>>();

    assert_eq!(reservations.len(), 1);

    assert!(slots.iter().all(|slot| slot.acquire().is_none()));

    drop(reservations);

    release.add_permits(WORKER_COUNT - 1);

    drop(slots);

    shutdown_workers(workers, StdDuration::from_secs(1)).await;
}

// unused_acquisitions_release_capacity(WorkerSlot::acquire)(negative): cancelled or partially filled claims return their unused slots.
#[test]
fn unused_reservations_release_capacity() {
    let (slot, _work_recv) = WorkerSlot::<()>::channel();

    let reservation = slot.acquire().unwrap();

    assert!(slot.acquire().is_none());

    drop(reservation);

    assert!(slot.acquire().is_some());
}

// failed_attempt_returns_reserved_capacity(WorkerSlot/run_worker)(negative): panic and timeout release permits for new claims.
#[tokio::test]
async fn failed_attempt_returns_reserved_capacity() {
    for should_panic in [true, false] {
        let (slot, work_recv) = WorkerSlot::channel();

        let completed = Arc::new(Notify::new());

        let started = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let observed_started = started.clone();

        let mut workers = JoinSet::new();

        workers.spawn(run_worker(work_recv, completed.clone(), move |()| {
            let started = started.clone();

            async move {
                started.store(true, std::sync::atomic::Ordering::SeqCst);

                assert!(!should_panic, "injected attempt panic");

                std::future::pending::<()>().await;
            }
        }));

        let permit = slot.acquire().unwrap();

        assert!(slot.dispatch(
            (),
            Instant::now() + StdDuration::from_millis(100),
            permit
        ));

        timeout(StdDuration::from_secs(1), completed.notified())
            .await
            .unwrap();

        assert!(observed_started.load(std::sync::atomic::Ordering::SeqCst));

        assert!(slot.acquire().is_some());

        drop(slot);

        shutdown_workers(workers, StdDuration::from_secs(1)).await;
    }
}
