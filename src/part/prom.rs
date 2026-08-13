//! Deferred-action producer port.

/// Deferred-action payloads.
pub mod payload;

use poprako_orchestra::Step;
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};

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
pub trait Prom<C>:
    for<'a> Step<Defer<'a, String, TaskPayload, ()>, C, Error = BaseError>
    + for<'t, 'a> Step<
        DeferBatch<'t, 'a, String, TaskPayload, ()>,
        C,
        Error = BaseError,
    >
{
}
