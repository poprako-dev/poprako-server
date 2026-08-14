//! Deferred-action producer port.

/// Deferred-action operation descriptors.
pub mod oper;
/// Deferred-action payloads.
pub mod payload;
/// Deferred-action task data.
pub mod task;

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
/// Delivery is at least once. A failed task may be delayed and consumed after
/// later tasks from the same topic, so producers and handlers must not rely on
/// `DeferBatch` order for correctness. Handlers must be idempotent and guard
/// state changes with the complete resource identity. Image confirmation, for
/// example, compares the resource id, monotonically increasing version, and
/// object key before marking an upload complete. Generated object keys must not
/// be reused by later resource versions.
#[drive(
    context = C,
    error = BaseError,
    step(
        for<'a> Defer<'a, String, TaskPayload, ()>,
        for<'t, 'a> DeferBatch<'t, 'a, String, TaskPayload, ()>,
    ),
)]
pub trait Prom<C> {}
