//! Fixed production composition for periodic background jobs.

// Comic archive retention periodic job.
mod comic_archive;

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::shared::RdbCore;

/// Owns the lifecycle of the fixed production periodic-job composition.
pub struct Sched {
    /// Shared cancellation signal for every explicitly composed job.
    token: CancellationToken,

    /// Completion receivers for graceful shutdown of the fixed job set.
    done_recvs: Vec<watch::Receiver<bool>>,
}

impl Sched {
    /// Starts the explicitly composed periodic jobs.
    pub fn new(core: RdbCore) -> Self {
        //
        let token = CancellationToken::new();

        let done_recvs = vec![comic_archive::spawn(core, token.clone())];

        Self { token, done_recvs }
    }

    /// Stops every periodic job and waits for in-flight work to finish.
    pub async fn close(&self) {
        //
        self.token.cancel();

        for done_recv in &self.done_recvs {
            //
            let mut done_recv = done_recv.clone();

            if let Err(error) = done_recv.wait_for(|done| *done).await {
                //
                tracing::error!(
                    err = %error,
                    "[GeneralSched::close] job ended without completion",
                );
            }
        }
    }
}

impl Drop for Sched {
    // Cancels the scheduler token on drop to stop periodic jobs.
    fn drop(&mut self) {
        self.token.cancel();
    }
}
