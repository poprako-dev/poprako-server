use anyhow::Context;
use tokio::net::{TcpListener, ToSocketAddrs};

use crate::api::harness::Harness;
use crate::api::http::router;

pub async fn serve<A>(harn: Harness, addr: A) -> anyhow::Result<()>
where
    A: ToSocketAddrs + std::fmt::Debug,
{
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("[server::serve] failed to bind listener on {:?}", addr))?;

    tracing::info!("[server::serve] listening on {:?}", addr);

    let app = router::new().with_state(harn);

    axum::serve(listener, app)
        .await
        .with_context(|| "[server::serve] server error")
}
