//! HTTP server bootstrap and graceful shutdown.

use std::net::SocketAddr;

use anyhow::Context as _;

use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::signal;

use crate::api::http::router;
use crate::api::http::state::AppHarn;

/// Installs Ctrl+C and (on unix) SIGTERM handlers that stop the server.
async fn shutdown_signal() {
    //
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    {
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler")
                .recv()
                .await;
        };

        tokio::select! {
            () = ctrl_c => {},
            () = terminate => {},
        }
    }
    #[cfg(not(unix))]
    {
        ctrl_c.await;
    }

    tracing::info!("shutdown signal received, starting graceful shutdown");
}

/// Binds `addr`, builds the router, and serves until shutdown.
pub async fn serve<A>(harn: AppHarn, addr: A) -> anyhow::Result<()>
where
    A: ToSocketAddrs + std::fmt::Debug,
{
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind listener on {:?}", addr))?;

    tracing::info!(addr = ?addr, "listening");

    let app = router::new(harn.clone()).with_state(harn);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .with_context(|| "server error")
}
