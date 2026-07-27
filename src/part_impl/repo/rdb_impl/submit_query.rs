// Allocates a connection and calls a free query function.
//
// The function must accept `(&mut AsyncPgConnection, args...)` and return
// a `Future<Output = Result<T, crate::result::Error>>`.
// TODO: not to place here.
macro_rules! submit_query {
    ($core:expr, $fn:path $(, $arg:expr)* $(,)?) => {{
        let mut conn = $core.get().await?;
        $fn(&mut *conn, $($arg),*).await
    }};
}
