//! Deferred-action operation descriptors.

use std::marker::PhantomData;

use poprako_orchestra::Oper;

use crate::part::prom::task::Task;

/// Persists one deferred action.
pub struct Defer<'a, I, P, O>
where
    I: AsRef<str>,
    P: ?Sized,
{
    /// Task to persist.
    pub task: Task<'a, I, P>,
    /// Operation output marker.
    output: PhantomData<O>,
}

impl<'a, I, P, O> Defer<'a, I, P, O>
where
    I: AsRef<str>,
    P: ?Sized,
{
    /// Builds a single-task operation.
    pub const fn new(task: Task<'a, I, P>) -> Self {
        //
        Self {
            task,
            output: PhantomData,
        }
    }
}

impl<I, P, O> Oper for Defer<'_, I, P, O>
where
    I: AsRef<str>,
    P: ?Sized,
{
    // Declares the single-task operation output.
    type Output = O;
}

/// Persists multiple deferred actions atomically.
pub struct DeferBatch<'t, 'a, I, P, O>
where
    I: AsRef<str>,
    P: ?Sized,
{
    /// Tasks to persist.
    pub tasks: &'t [Task<'a, I, P>],
    /// Operation output marker.
    output: PhantomData<O>,
}

impl<'t, 'a, I, P, O> DeferBatch<'t, 'a, I, P, O>
where
    I: AsRef<str>,
    P: ?Sized,
{
    /// Builds a batch operation.
    pub const fn new(tasks: &'t [Task<'a, I, P>]) -> Self {
        //
        Self {
            tasks,
            output: PhantomData,
        }
    }
}

impl<I, P, O> Oper for DeferBatch<'_, '_, I, P, O>
where
    I: AsRef<str>,
    P: ?Sized,
{
    // Declares the batch operation output.
    type Output = O;
}
