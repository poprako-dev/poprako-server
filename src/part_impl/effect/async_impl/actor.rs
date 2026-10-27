//! Background event actor — receives events from the channel and dispatches
//! them to the appropriate domain actor.

use poprako_orchestra::Context;
use tokio::sync::mpsc::Receiver;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::part::effect::event::Event;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::system_mail::SystemMailRepo;
use crate::part::repo::team::TeamRepo;
use crate::part_impl::effect::async_impl::dispatch::dispatch;

/// Unique stream capability for a bounded event queue.
pub struct EventBusStream {
    /// Stream transferred to the event actor.
    stream: Receiver<Event>,
}

impl EventBusStream {
    /// Creates a stream capability for the event actor.
    #[must_use]
    pub const fn new(stream: Receiver<Event>) -> Self {
        Self { stream }
    }

    /// Transfers the stream to the actor.
    #[must_use]
    pub fn into_stream(self) -> Receiver<Event> {
        self.stream
    }
}

/// Owns cancellation and completion of one background supervisor.
pub struct EventBusActorDesc {
    /// Cancellation signal for the supervisor.
    token: CancellationToken,
    /// Task whose completion includes its worker shutdown.
    task: tokio::task::JoinHandle<()>,
}

impl EventBusActorDesc {
    /// Requests cancellation without waiting for completion.
    pub fn cancel(&self) {
        self.token.cancel();
    }

    /// Waits for completion and reports a supervisor panic or cancellation.
    ///
    /// # Errors
    /// Returns the supervisor task's join error.
    pub async fn join(mut self) -> Result<(), tokio::task::JoinError> {
        (&mut self.task).await
    }

    /// Requests cancellation and waits for shutdown to complete.
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

impl Drop for EventBusActorDesc {
    // Request shutdown when the owner is dropped without joining.
    fn drop(&mut self) {
        self.token.cancel();
    }
}

/// Background event consumer that receives events from the channel and
/// dispatches them to the appropriate domain actor.
pub struct EventBusActor<R> {
    /// Shared repository access for event processing.
    repo: R,
    /// Channel stream that yields queued events.
    stream: Receiver<Event>,
    /// Cancellation token to stop the background loop.
    token: CancellationToken,
}

impl<R> EventBusActor<R> {
    /// Constructs a consumer with its injected repository and unique stream.
    pub fn new(repo: R, event_bus_stream: EventBusStream) -> Self {
        Self {
            repo,
            stream: event_bus_stream.into_stream(),
            token: CancellationToken::new(),
        }
    }

    /// Starts consumption and returns its unique runtime owner.
    #[must_use]
    pub fn run_detached<C>(self) -> EventBusActorDesc
    where
        C: Context + Send + 'static,
        R: AssignmentRepo<C>
            + ChapterRepo<C>
            + TeamRepo<C>
            + SystemMailRepo
            + Send
            + Sync
            + 'static,
    {
        let (token, task) = (self.token.clone(), tokio::spawn(self.run::<C>()));

        EventBusActorDesc { token, task }
    }

    #[instrument(level = "info", skip_all)]
    /// Runs the event consumer loop, dispatching events until a shutdown signal is received.
    pub async fn run<C>(mut self)
    where
        C: Context + Send,
        R: AssignmentRepo<C>
            + ChapterRepo<C>
            + TeamRepo<C>
            + SystemMailRepo
            + Send
            + Sync,
    {
        loop {
            //
            tokio::select! {
                //
                event = self.stream.recv() => {
                    //
                    match event {
                        //
                        Some(event) => {
                            dispatch::<C, R>(&self.repo, event).await;
                        }

                        None => break,
                    }
                }

                () = self.token.cancelled() => break,
            }
        }

        self.stream.close();

        while let Ok(event) = self.stream.try_recv() {
            dispatch::<C, R>(&self.repo, event).await;
        }
    }
}
