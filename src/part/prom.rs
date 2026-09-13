//! Deferred-action producer port.

/// Deferred-action operation descriptors.
pub mod oper;
/// Deferred-action payloads.
pub mod payload;
/// Deferred-action task data.
pub mod task;
/// Fixed consumption queues.
pub mod topic;

use poprako_orchestra::drive;

use crate::part::prom::oper::{Defer, DeferBatch};
use crate::part::prom::payload::TaskPayload;
use crate::result::BaseError;

/// Prom operations within a caller-coordinated transaction.
///
/// Implementors persist individual and batch tasks against the shared
/// transaction context `C` supplied by the application coordinator.
///
/// # Delivery contract
///
/// Delivery is at least once. Multiple payload kinds can share a topic.
/// Tasks in one topic run serially; different topics can execute concurrently.
/// Delayed retries may follow newer work. Use-case idempotence is not assumed:
/// each task kind needs a verified replay and recovery contract covering
/// business commits, side effects, and failed acknowledgements. Use cases own
/// their business transactions; queue acknowledgement is a separate operation.
///
/// Each task is retained independently, including tasks sharing a topic.
/// Batch order is not guaranteed.
#[drive(
    context = C,
    error = BaseError,
    step(
        for<'a> Defer<'a, String, TaskPayload, ()>,
        for<'t, 'a> DeferBatch<'t, 'a, String, TaskPayload, ()>,
    ),
)]
pub trait Prom<C> {}
