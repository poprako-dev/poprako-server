use super::*;

// topic_worker_assignment_is_stable(topic_worker_index)(positive): repeated messages for one topic must stay on one serial worker.
// current_topics_use_multiple_workers(topic_worker_index)(positive): the current topic set should not collapse onto one worker.

#[test]
fn topic_worker_assignment_is_stable() {
    //
    let Ok(first_worker) = topic_worker_index("advance_raw_provide") else {
        panic!("worker index calculation must succeed");
    };

    let Ok(second_worker) = topic_worker_index("advance_raw_provide") else {
        panic!("worker index calculation must succeed");
    };

    assert_eq!(first_worker, second_worker);
}

#[test]
fn current_topics_use_multiple_workers() {
    //
    let worker_results = [
        topic_worker_index("advance_raw_provide"),
        topic_worker_index("purge_expired_invitation"),
    ];

    let mut worker_indices = worker_results
        .into_iter()
        .map(|result| match result {
            Ok(worker_index) => worker_index,

            Err(_) => panic!("worker index calculation must succeed"),
        })
        .collect::<Vec<_>>();

    worker_indices.sort_unstable();

    worker_indices.dedup();

    assert!(worker_indices.len() >= 2);
}

// timed_out_attempt_releases_worker(run_worker)(negative): dropping a pending attempt frees its worker for the next task.
#[tokio::test]
async fn timed_out_attempt_releases_worker() {
    let (work_send, work_recv) = mpsc::unbounded_channel();

    let cancelled = tokio_util::sync::CancellationToken::new();

    let finished = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let deadline = Instant::now() + StdDuration::from_millis(100);

    work_send.send((true, deadline)).unwrap();

    work_send
        .send((false, Instant::now() + StdDuration::from_secs(1)))
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

// panicked_attempt_releases_worker(run_worker)(negative): a task panic leaves the shard available to subsequent tasks.
#[tokio::test]
async fn panicked_attempt_releases_worker() {
    let (work_send, work_recv) = mpsc::unbounded_channel();

    let finished = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let deadline = Instant::now() + StdDuration::from_secs(1);

    work_send.send((true, deadline)).unwrap();

    work_send.send((false, deadline)).unwrap();

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

    work_send.send(((), Instant::now())).unwrap();

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
