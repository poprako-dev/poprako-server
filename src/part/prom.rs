//! Deferred-action producer port.

use poprako_orchestra::Step;
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};

use crate::part::prom::payload::Payload;
use crate::result::RegularError;

/// Deferred-action payloads.
pub mod payload;

/// Prom operations within a caller-coordinated transaction.
///
/// Implementors persist individual and batch tasks against the shared
/// transaction context `C` supplied by the application coordinator.
///
/// # SAFETY
///
/// `DeferBatch` implementations **must** record every task in the exact
/// order of the given slice. Callers rely on insertion order to express
/// causal dependencies — for example a delete task that must be processed
/// before the check-upload task that replaces the same resource. An
/// implementation that reorders, interleaves, or drops tasks violates
/// this contract and may cause orphaned storage objects or stale state.
pub trait Prom<C>:
    for<'a> Step<Defer<'a, String, Payload, ()>, C, Error = RegularError>
    + for<'t, 'a> Step<
        DeferBatch<'t, 'a, String, Payload, ()>,
        C,
        Error = RegularError,
    >
{
}
