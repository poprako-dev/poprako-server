#[cfg(test)]
mod tests;

use std::future::Future;

use futures_util::stream::{StreamExt as _, TryStreamExt as _, iter};
use poprako_orchestra::Context;

use crate::result::BaseRest;

// Resolves up to this many futures concurrently while preserving order.
const FUTURE_CONCURRENCY_LIMIT: usize = 20;

/// Resolves fallible futures with bounded concurrency while preserving input order.
pub async fn collect_bounded<F, I, T>(futures: I) -> BaseRest<Vec<T>>
where
    F: Future<Output = BaseRest<T>>,
    I: IntoIterator<Item = F>,
{
    iter(futures)
        .buffered(FUTURE_CONCURRENCY_LIMIT)
        .try_collect::<Vec<_>>()
        .await
}

/// Selects whether a loader executes independently or in a caller transaction.
pub enum LoadMode<'a, C>
where
    C: Context,
{
    /// Executes the operation independently.
    Run,

    /// Executes the operation through the caller-owned transaction context.
    Step {
        /// The active transaction context.
        context: &'a mut C,
    },
}
