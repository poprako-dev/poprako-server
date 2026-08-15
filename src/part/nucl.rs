//! Transaction-coordinator boundary error conversion.

use std::future::Future;

use poprako_orchestra::nucl::Error as NuclError;
use poprako_orchestra::{AtLeast, Context, Level, LevelGuard, Oper, Step};

use crate::result::BaseError;

/// PostgreSQL repeatable-read transaction guarantee.
pub struct RepeatableRead;

impl Level for RepeatableRead {}

/// PostgreSQL serializable transaction guarantee.
pub struct Serializable;

impl Level for Serializable {}

impl AtLeast<RepeatableRead> for Serializable {}

/// Adapts a capability-guarded stepper for Orchestra's step proxy.
///
/// Orchestra's aggregate repository traits already carry the operation-level
/// guards. This adapter preserves those guards while presenting the active
/// context level to the proxy implementation.
pub struct GuardedStep<'a, R>(&'a R)
where
    R: ?Sized;

impl<'a, R> GuardedStep<'a, R>
where
    R: ?Sized,
{
    /// Wraps a repository whose aggregate capability proves its step levels.
    pub fn new(repo: &'a R) -> Self {
        Self(repo)
    }
}

impl<O, C, R> Step<O, C> for GuardedStep<'_, R>
where
    O: Oper,
    C: Context,
    R: Step<O, C>
        + LevelGuard<C::Level, <R as Step<O, C>>::Level>
        + Sync
        + ?Sized,
{
    // Uses the active context level after the aggregate guard is proven.
    type Level = C::Level;

    // Preserves the wrapped repository error.
    type Error = <R as Step<O, C>>::Error;

    // Delegates the operation to the wrapped repository.
    fn step(
        &self,
        context: &mut C,
        oper: &O,
    ) -> impl Future<Output = Result<O::Output, Self::Error>> + Send {
        self.0.step(context, oper)
    }
}

impl<BE, E> From<NuclError<BE, E>> for BaseError
where
    BE: Into<BaseError>,
    E: Into<BaseError>,
{
    // Converts a Nucl error into an application-level Error, unwrapping the backend or step inner error.
    fn from(value: NuclError<BE, E>) -> Self {
        //
        match value {
            //
            NuclError::Backend(error) => error.into(),

            NuclError::Step(error) => error.into(),
        }
    }
}
