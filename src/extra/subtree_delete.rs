//! Relational hierarchy sweep task for the generic scheduler.

#[cfg(test)]
mod tests;

use std::future::Future;
use std::num::NonZeroUsize;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use poprako_obj_dept::ObjDept;

use crate::config::sched::SchedTaskConfig;
use crate::extra::sched::{SchedNext, SchedTask, SchedTaskRunner};
use crate::part::obj_dept::{
    ChapterArtwork, ComicCover, PageImage, TeamAvatar,
};
use crate::part::repo::subtree_delete::SubtreeRepo;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::result::{BaseRest, accept};
use crate::shared::RdbContext;
use crate::usecase::subtree_delete as subtree_delete_usecase;
use crate::value::subtree_delete::SubtreeSweepLevel;

// Hierarchy levels are polled from leaves to roots.
const SWEEP_LEVELS: [SubtreeSweepLevel; 4] = [
    SubtreeSweepLevel::Chapter,
    SubtreeSweepLevel::Comic,
    SubtreeSweepLevel::Workset,
    SubtreeSweepLevel::Team,
];

/// Creates independent sweep runners using explicitly injected business ports.
pub struct SubtreeDeleteTask<R, O> {
    /// Transaction coordinator shared through its connection pool.
    nucl: RdbNucl,
    /// Business repository shared by the application.
    repo: R,
    /// Object lifecycle port participating in each sweep transaction.
    obj_dept: O,
}

impl<R, O> SubtreeDeleteTask<R, O> {
    /// Constructs a task without starting background work.
    #[must_use]
    pub const fn new(nucl: RdbNucl, repo: R, obj_dept: O) -> Self {
        Self {
            nucl,
            repo,
            obj_dept,
        }
    }
}

impl<R, O> SchedTask for SubtreeDeleteTask<R, O>
where
    R: SubtreeRepo<RdbContext> + Clone + Send + Sync + 'static,
    O: ObjDept<ChapterArtwork, RdbContext>
        + ObjDept<PageImage, RdbContext>
        + ObjDept<ComicCover, RdbContext>
        + ObjDept<TeamAvatar, RdbContext>
        + Clone
        + Send
        + Sync
        + 'static,
{
    // Retain two concurrent sweep workers with the scheduler's default timing.
    fn config(&self) -> SchedTaskConfig {
        SchedTaskConfig {
            name: "subtree_delete",
            concurrency: NonZeroUsize::MIN.saturating_add(1),
            ..SchedTaskConfig::default()
        }
    }

    // Clone ports once per runner rather than once per hierarchy level.
    fn runner(&self) -> Box<dyn SchedTaskRunner + Send> {
        //
        Box::new(SubtreeDeleteTaskRunner {
            nucl: self.nucl.clone(),
            repo: self.repo.clone(),
            obj_dept: self.obj_dept.clone(),
        })
    }
}

// One worker's sweep ports; transaction ownership remains in the use case.
struct SubtreeDeleteTaskRunner<R, O> {
    // Transaction coordinator for this runner.
    nucl: RdbNucl,
    // Shared repository handle.
    repo: R,
    // Shared object lifecycle handle.
    obj_dept: O,
}

#[async_trait]
impl<R, O> SchedTaskRunner for SubtreeDeleteTaskRunner<R, O>
where
    R: SubtreeRepo<RdbContext> + Send + Sync,
    O: ObjDept<ChapterArtwork, RdbContext>
        + ObjDept<PageImage, RdbContext>
        + ObjDept<ComicCover, RdbContext>
        + ObjDept<TeamAvatar, RdbContext>
        + Send
        + Sync,
{
    // Run one round; the generic scheduler owns repetition and shutdown timing.
    async fn run(&mut self, token: CancellationToken) -> BaseRest<SchedNext> {
        //
        let swept = sweep(&token, |level| {
            //
            subtree_delete_usecase::sweep(
                (&self.nucl, &self.repo, &self.obj_dept),
                level,
            )
        })
        .await;

        if swept {
            return accept(SchedNext::Continue);
        }

        accept(SchedNext::Wait)
    }
}

// Poll one hierarchy sweep round in dependency order.
async fn sweep<F, Fut>(token: &CancellationToken, mut sweep_level: F) -> bool
where
    F: FnMut(SubtreeSweepLevel) -> Fut,
    Fut: Future<Output = BaseRest<bool>>,
{
    for sweep_level_value in SWEEP_LEVELS {
        //
        if token.is_cancelled() {
            return false;
        }

        match sweep_level(sweep_level_value).await {
            //
            Ok(true) => return true,

            Ok(false) => {}

            Err(_) => {
                //
                tracing::warn!(
                    sweep_level = ?sweep_level_value,
                    operation = "sweep_subtree_delete",
                    "hierarchy sweep level failed and polling will continue",
                );
            }
        }
    }

    false
}
