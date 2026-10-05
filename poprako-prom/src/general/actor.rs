// Polling supervisors and topic worker coordination.
mod pool;
// Bounded worker execution and shutdown.
mod worker;

#[cfg(test)]
mod tests;

use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::general::delivery::Delivery;
use crate::general::dispatch_flow::DispatchFlow;
use crate::general::handler::{Handler, Payload};

/// Owns cancellation and completion of one background supervisor.
pub struct PromActorDesc {
    /// Cancellation signal for the supervisor.
    token: CancellationToken,
    /// Supervisor task that drains workers with a grace period, then aborts.
    task: tokio::task::JoinHandle<()>,
}

impl PromActorDesc {
    /// Requests cancellation without waiting for completion.
    pub fn cancel(&self) {
        self.token.cancel();
    }

    /// Waits for the supervisor's bounded shutdown and reports join failures.
    /// Workers that exceed the grace period are aborted; non-yielding code
    /// cannot be forcibly stopped by the async runtime.
    ///
    /// # Errors
    /// Returns the supervisor task's join error.
    pub async fn join(mut self) -> Result<(), tokio::task::JoinError> {
        (&mut self.task).await
    }

    /// Requests cancellation and waits for the supervisor's bounded shutdown.
    ///
    /// # Errors
    /// Returns the supervisor task's join error.
    pub async fn cancel_and_join(
        mut self,
    ) -> Result<(), tokio::task::JoinError> {
        //
        self.token.cancel();

        (&mut self.task).await
    }
}

impl Drop for PromActorDesc {
    // Request shutdown when the owner is dropped without joining.
    fn drop(&mut self) {
        self.token.cancel();
    }
}

/// Actor composed from a delivery interface and an associated-payload handler.
/// Construction does not start background work.
pub struct PromActor<P, H> {
    /// Persistence access for claiming and finalizing attempts.
    delivery: P,
    /// Injected business dispatcher.
    handler: H,
    /// Cancellation signal for the supervisor.
    token: CancellationToken,
}

impl<P, H> PromActor<P, H> {
    /// Constructs an idle consumer from already composed dependencies.
    pub fn new(delivery: P, handler: H) -> Self {
        Self {
            delivery,
            handler,
            token: CancellationToken::new(),
        }
    }
}

impl<P, H> PromActor<P, H>
where
    H: Handler + Send + Sync + 'static,
    H::Payload: Send,
{
    /// Decodes and validates one envelope before invoking its typed handler.
    /// Task acknowledgement remains the responsibility of the running actor.
    #[instrument(level = "info", skip_all)]
    #[expect(
        clippy::future_not_send,
        clippy::uninlined_format_args,
        reason = "Direct dispatch supports local delivery contexts and keeps interpolation arguments explicit"
    )]
    pub async fn dispatch_payload(
        &self,
        topic: &str,
        payload: &serde_json::Value,
    ) -> DispatchFlow {
        //
        let task = match serde_json::from_value::<H::Payload>(payload.clone()) {
            //
            Ok(task) => task,

            Err(error) => {
                //
                tracing::error!(
                    operation = "deserialize_prom_payload",
                    sdk_err = ?error,
                    "JSON SDK deserialization error",
                );

                return DispatchFlow::Dead {
                    err_msg: format!(
                        "failed to deserialize prom payload: {}",
                        error
                    ),
                };
            }
        };

        let expected_topic = task.topic();

        if topic != expected_topic || !H::Payload::TOPICS.contains(&topic) {
            return DispatchFlow::Dead {
                err_msg: format!(
                    "prom topic {} does not match registered payload topic {}",
                    topic, expected_topic,
                ),
            };
        }

        self.handler.handle(task).await
    }
}

impl<P, H> PromActor<P, H>
where
    P: Delivery + Send + Sync + 'static,
    H: Handler + Send + Sync + 'static,
    H::Payload: Send,
{
    /// Starts the consumer and transfers shutdown ownership to its descriptor.
    #[must_use]
    pub fn run_detached(self) -> PromActorDesc {
        //
        let token = self.token.clone();

        let task = tokio::spawn(self.run());

        PromActorDesc { token, task }
    }
}
