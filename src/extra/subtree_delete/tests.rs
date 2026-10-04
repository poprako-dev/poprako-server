//! Hierarchy sweep policy tests independent of scheduling mechanics.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio_util::sync::CancellationToken;

use crate::result::{BaseError, BaseRest};
use crate::value::subtree_delete::SubtreeSweepLevel;

use super::sweep;

fn failure() -> BaseError {
    BaseError::Unrecoverable {
        msg: "injected sweep failure".into(),
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

// chapter_success_stops_the_round(sweep)(positive): successful leaf work ends the round immediately.
#[tokio::test]
async fn chapter_success_stops_the_round() {
    let (swept, levels) = run_sweep(vec![Ok(true)]).await;

    assert!(swept);

    assert_eq!(levels, vec![SubtreeSweepLevel::Chapter]);
}

// chapter_failure_falls_through_to_comic_success(sweep)(negative): a failed level does not prevent other levels from making progress.
#[tokio::test]
async fn chapter_failure_falls_through_to_comic_success() {
    let (swept, levels) = run_sweep(vec![Err(failure()), Ok(true)]).await;

    assert!(swept);

    assert_eq!(
        levels,
        vec![SubtreeSweepLevel::Chapter, SubtreeSweepLevel::Comic]
    );
}

// empty_levels_fall_through_in_order(sweep)(positive): hierarchy polling proceeds from leaves towards roots.
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

// unsuccessful_round_returns_idle(sweep)(negative): empty and failed levels produce an idle decision after the complete round.
#[tokio::test]
async fn unsuccessful_round_returns_idle() {
    let (swept, levels) =
        run_sweep(vec![Ok(false), Err(failure()), Ok(false), Err(failure())])
            .await;

    assert!(!swept);

    assert_eq!(levels.len(), 4);

    assert_eq!(levels.last(), Some(&SubtreeSweepLevel::Team));
}

// lower_level_success_restarts_from_chapter(sweep)(positive): the next independently invoked round restarts at the leaf level.
#[tokio::test]
async fn lower_level_success_restarts_from_chapter() {
    let (swept, levels) = run_sweep(vec![Ok(false), Ok(true)]).await;

    assert!(swept);

    assert_eq!(
        levels,
        vec![SubtreeSweepLevel::Chapter, SubtreeSweepLevel::Comic]
    );

    let (swept, levels) = run_sweep(vec![Ok(true)]).await;

    assert!(swept);

    assert_eq!(levels, vec![SubtreeSweepLevel::Chapter]);
}

// cancellation_prevents_claiming_the_next_level(sweep)(negative): shutdown lets the current level finish and prevents another claim.
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
