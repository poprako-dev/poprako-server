use anyhow::Context;
use std::fmt::Debug;
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::api::http::router;
use crate::harness::Harness;

pub async fn serve<A>(harn: Harness, addr: A) -> anyhow::Result<()>
where
    A: ToSocketAddrs + Debug,
{
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("[server::serve] failed to bind listener on {:?}", addr))?;

    tracing::info!(addr = ?addr, "[server::serve] listening");

    let app = router::new(harn.clone()).with_state(harn);

    axum::serve(listener, app)
        .await
        .with_context(|| "[server::serve] server error")
}
