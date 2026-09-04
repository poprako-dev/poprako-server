//! Scheduler policy tests for hierarchy sweeping.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio_util::sync::CancellationToken;

use crate::result::{BaseError, BaseRest};
use crate::value::subtree_delete::SubtreeSweepLevel;

use super::{RETRY_DELAY, run, sweep, wait};

fn failure() -> BaseError {
    BaseError::Unrecoverable {
        message: "injected sweep failure".into(),
    }
}

async fn run_sweep(
    outcomes: Vec<BaseRest<bool>>,
) -> (bool, Vec<SubtreeSweepLevel>) {
    let token = CancellationToken::new();
    let outcomes = Arc::new(Mutex::new(VecDeque::from(outcomes)));
    let levels = Arc::new(Mutex::new(Vec::new()));

    let swept = sweep(&token, {
        let outcomes = outcomes.clone();
        let levels = levels.clone();

        move |level| {
            levels.lock().unwrap().push(level);

            let outcome = outcomes.lock().unwrap().pop_front().unwrap();

            async move { outcome }
        }
    })
    .await;

    let levels = levels.lock().unwrap().clone();

    (swept, levels)
}

#[tokio::test]
async fn chapter_success_stops_the_round() {
    let (swept, levels) = run_sweep(vec![Ok(true)]).await;

    assert!(swept);
    assert_eq!(levels, vec![SubtreeSweepLevel::Chapter]);
}

#[tokio::test]
async fn chapter_failure_falls_through_to_comic_success() {
    let (swept, levels) = run_sweep(vec![Err(failure()), Ok(true)]).await;

    assert!(swept);
    assert_eq!(
        levels,
        vec![SubtreeSweepLevel::Chapter, SubtreeSweepLevel::Comic]
    );
}

#[tokio::test]
async fn empty_levels_fall_through_in_order() {
    let (swept, levels) = run_sweep(vec![Ok(false), Ok(false), Ok(true)]).await;

    assert!(swept);
    assert_eq!(
        levels,
        vec![
            SubtreeSweepLevel::Chapter,
            SubtreeSweepLevel::Comic,
            SubtreeSweepLevel::Workset,
        ]
    );
}

#[tokio::test]
async fn unsuccessful_round_enters_retry_wait() {
    let token = CancellationToken::new();
    let levels = Arc::new(Mutex::new(Vec::new()));
    let waits = Arc::new(Mutex::new(0));
    let outcomes = Arc::new(Mutex::new(VecDeque::from(vec![
        Ok(false),
        Err(failure()),
        Ok(false),
        Err(failure()),
    ])));

    run(
        &token,
        {
            let levels = levels.clone();
            let outcomes = outcomes.clone();

            move |level| {
                levels.lock().unwrap().push(level);

                let outcome = outcomes.lock().unwrap().pop_front().unwrap();

                async move { outcome }
            }
        },
        {
            let waits = waits.clone();

            move || {
                *waits.lock().unwrap() += 1;

                async { true }
            }
        },
    )
    .await;

    assert_eq!(levels.lock().unwrap().len(), 4);
    assert_eq!(*waits.lock().unwrap(), 1);
}

#[tokio::test]
async fn lower_level_success_restarts_from_chapter() {
    let token = CancellationToken::new();
    let levels = Arc::new(Mutex::new(Vec::new()));
    let outcomes = Arc::new(Mutex::new(VecDeque::from(vec![
        Ok(false),
        Ok(true),
        Ok(true),
    ])));

    run(
        &token,
        {
            let token = token.clone();
            let levels = levels.clone();
            let outcomes = outcomes.clone();

            move |level| {
                let mut levels = levels.lock().unwrap();

                levels.push(level);

                if levels.len() == 3 {
                    token.cancel();
                }

                drop(levels);

                let outcome = outcomes.lock().unwrap().pop_front().unwrap();

                async move { outcome }
            }
        },
        || async { false },
    )
    .await;

    assert_eq!(
        *levels.lock().unwrap(),
        vec![
            SubtreeSweepLevel::Chapter,
            SubtreeSweepLevel::Comic,
            SubtreeSweepLevel::Chapter,
        ]
    );
}

#[tokio::test]
async fn retry_wait_uses_five_seconds_and_is_cancellable() {
    assert_eq!(RETRY_DELAY, std::time::Duration::from_secs(5));

    let token = CancellationToken::new();
    let wait_future = wait(&token);

    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(10), wait_future)
            .await
            .is_err()
    );

    token.cancel();

    assert!(wait(&token).await);
}

#[tokio::test]
async fn cancellation_prevents_claiming_the_next_level() {
    let token = CancellationToken::new();
    let levels = Arc::new(Mutex::new(Vec::new()));

    let swept = sweep(&token, {
        let token = token.clone();
        let levels = levels.clone();

        move |level| {
            levels.lock().unwrap().push(level);

            token.cancel();

            async { Ok(false) }
        }
    })
    .await;

    assert!(!swept);
    assert_eq!(*levels.lock().unwrap(), vec![SubtreeSweepLevel::Chapter]);
}
