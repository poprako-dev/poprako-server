// Allocates a connection and calls a free query function.
//
// The function must accept `(&mut AsyncPgConnection, args...)` and return
// a future returning the application BaseRest. This macro remains private to
// the RDB adapter so connection acquisition stays outside domain use cases.
macro_rules! submit_query {
    ($core:expr, $fn:path $(, $arg:expr)* $(,)?) => {{
        let mut conn = $core.get().await?;
        $fn(&mut *conn, $($arg),*).await
    }};
}
