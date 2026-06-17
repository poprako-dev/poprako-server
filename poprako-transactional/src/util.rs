pub trait AsyncFnMark<T, R>:
    AsyncFnOnce(T) -> R + FnOnce(T) -> <Self as AsyncFnMark<T, R>>::Fut
{
    type Fut: Future<Output = R>;
}

impl<F, T, Fut, R> AsyncFnMark<T, R> for F
where
    F: AsyncFnOnce(T) -> R + FnOnce(T) -> Fut,
    Fut: Future<Output = R>,
{
    type Fut = Fut;
}
