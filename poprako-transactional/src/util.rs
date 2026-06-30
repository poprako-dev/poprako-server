//! Provides the [`AsyncFnMark`] trait, a helper that bridges stable Rust's
//! [`AsyncFnOnce`] with the legacy [`FnOnce`] bound needed by the [`Drive`]
//! trait.

/// A marker trait extending [`AsyncFnOnce`] with the equivalent [`FnOnce`]
/// bound, enabling higher-ranked async functions in trait bounds.
///
/// This is a workaround for Rust's current inability to express
/// `for<'a> AsyncFnOnce(&'a mut C) -> Result<T, E>` directly in trait
/// bounds without also naming the future type.
pub trait AsyncFnMark<T, R>:
    AsyncFnOnce(T) -> R + FnOnce(T) -> <Self as AsyncFnMark<T, R>>::Fut
{
    /// The concrete future type returned by the async function.
    type Fut: Future<Output = R>;
}

impl<F, T, Fut, R> AsyncFnMark<T, R> for F
where
    F: AsyncFnOnce(T) -> R + FnOnce(T) -> Fut,
    Fut: Future<Output = R>,
{
    type Fut = Fut;
}
