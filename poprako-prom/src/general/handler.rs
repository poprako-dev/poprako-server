use std::future::Future;
use std::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::general::dispatch_flow::DispatchFlow;

/// Serializable business payload and its stable consumption queues.
pub trait Payload: Serialize + DeserializeOwned {
    /// Distinct, nonempty topic names serviced by this payload's handler.
    const TOPICS: &'static [&'static str];

    /// Returns one of `TOPICS` for this payload.
    fn topic(&self) -> &'static str;
}

/// Business delivery policy for exactly one associated payload type.
///
/// Calls for different topics may overlap. Each call owns its payload; handler
/// state is shared. Returned futures must be safe to execute on Tokio workers.
pub trait Handler {
    /// Payload decoded and validated before this handler is invoked.
    type Payload: Payload;

    /// Runs business work and selects completion, retry, wait, or dead-lettering.
    fn handle(
        &self,
        payload: Self::Payload,
    ) -> impl Future<Output = DispatchFlow> + Send;
}

/// Dependency injection through a dispatcher.
///
/// Dependencies are cloned by the dispatcher before invoking the function.
///
/// ```
/// use poprako_prom::general::handler::{Dispatcher, Payload};
/// use poprako_prom::general::actor::PromActor;
/// use poprako_prom::general::dispatch_flow::DispatchFlow;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Refresh { id: String }
///
/// impl Payload for Refresh {
///     const TOPICS: &'static [&'static str] = &["refresh"];
///
///     fn topic(&self) -> &'static str { "refresh" }
/// }
///
/// let handler = Dispatcher::new((), |(), payload: Refresh| async move {
///     let _id = payload.id;
///
///     DispatchFlow::Complete
/// });
///
/// // Replace () with a Delivery implementation before calling run_detached().
/// let _actor = PromActor::new((), handler);
/// ```
pub struct Dispatcher<P, D, F> {
    /// Dependencies cloned for each dispatch call.
    deps: D,
    /// Injected business dispatcher.
    handler: F,
    /// Associates the dispatcher with its payload type.
    payload: PhantomData<fn(P)>,
}

impl<P, D, F, Fut> Dispatcher<P, D, F>
where
    P: Payload,
    D: Clone + Send + Sync + 'static,
    F: Fn(D, P) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = DispatchFlow> + Send,
{
    /// Owns injected dependencies and passes a clone to each dispatch call.
    pub const fn new(deps: D, handler: F) -> Self {
        Self {
            deps,
            handler,
            payload: PhantomData,
        }
    }
}

impl<P, D, F, Fut> Handler for Dispatcher<P, D, F>
where
    P: Payload,
    D: Clone + Send + Sync + 'static,
    F: Fn(D, P) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = DispatchFlow> + Send,
{
    // Business payload accepted by this dispatcher.
    type Payload = P;

    // Dispatches the payload using cloned dependencies.
    fn handle(&self, payload: P) -> impl Future<Output = DispatchFlow> + Send {
        (self.handler)(self.deps.clone(), payload)
    }
}
