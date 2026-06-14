use futures::future::BoxFuture;

pub type DynFut<'a, T> = BoxFuture<'a, T>;
