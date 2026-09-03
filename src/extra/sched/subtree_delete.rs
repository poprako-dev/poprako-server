//! Periodic relational hierarchy sweeping.

use std::time::Duration;

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use poprako_obj_dept::ObjDept;
use poprako_rdb_core::RdbCore;

use crate::part::nucl::ReptRead;
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase;

// Delay between empty or failed sweep attempts.
const RETRY_DELAY: Duration = Duration::from_secs(5);

/// Wait for cancellation or the next retry interval.
pub async fn wait(token: &CancellationToken) -> bool {
    //
    tokio::select! {
        () = token.cancelled() => true,
        () = tokio::time::sleep(RETRY_DELAY) => false,
    }
}

/// Spawns one hierarchy sweep worker.
pub fn spawn<O>(
    core: RdbCore,
    obj_dept: O,
    token: CancellationToken,
) -> watch::Receiver<bool>
where
    O: ObjDept<PageImage, RdbContext<ReptRead>>
        + ObjDept<ComicCover, RdbContext<ReptRead>>
        + ObjDept<TeamAvatar, RdbContext<ReptRead>>
        + Send
        + Sync
        + 'static,
{
    let (done_send, done_recv) = watch::channel(false);

    tokio::spawn(async move {
        //
        let nucl = RdbNucl::<ReptRead>::new(core.clone());

        let repo = HybRepo::new(core);

        loop {
            //
            if token.is_cancelled() {
                break;
            }

            let result =
                usecase::subtree_delete::sweep_once((&nucl, &repo, &obj_dept))
                    .await;

            match result {
                //
                Ok(true) => {}

                Ok(false) => {
                    //
                    if wait(&token).await {
                        break;
                    }
                }

                Err(error) => {
                    // FIXME: add bounded backoff and persistent-failure
                    // quarantine once operational policy is defined.
                    tracing::error!(
                        err = ?error,
                        operation = "sweep_subtree_delete",
                        "hierarchy sweep failed and will be retried",
                    );

                    if wait(&token).await {
                        break;
                    }
                }
            }
        }

        done_send.send_replace(true);
    });

    done_recv
}
