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
pub trait Prom<C>:
    for<'a> Step<Defer<'a, String, Payload, ()>, C, Error = RegularError>
    + for<'t, 'a> Step<
        DeferBatch<'t, 'a, String, Payload, ()>,
        C,
        Error = RegularError,
    >
{
}
