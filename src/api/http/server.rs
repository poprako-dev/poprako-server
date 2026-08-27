//! HTTP server bootstrap and graceful shutdown.

use std::future::pending;
use std::net::SocketAddr;

use anyhow::Context as _;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::signal;

use crate::api::http::router;
use crate::api::http::shared::prometheus::init_prometheus;
use crate::api::http::state::AppHarn;

/// Installs Ctrl+C and (on unix) SIGTERM handlers that stop the server.
///
/// Binds `addr`, builds the router, and serves until shutdown.
/// Returns an error when metrics initialization, listener binding, or serving
/// fails.
///
/// # Errors
///
/// Returns an error when metrics initialization, listener binding, or serving
/// fails.
pub async fn serve<A>(harn: AppHarn, addr: A) -> anyhow::Result<()>
where
    A: ToSocketAddrs + std::fmt::Debug,
{
    init_prometheus()?;

    let listener = TcpListener::bind(&addr)
        .await
        .inspect_err(|error| {
            //
            tracing::error!(
                operation = "bind_http_listener",
                sdk_err = ?error,
                "Tokio SDK listener bind error",
            );
        })
        .with_context(|| format!("failed to bind listener on {:?}", addr))?;

    tracing::info!(addr = ?addr, "listening");

    let app = router::new(harn.clone()).with_state(harn);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .inspect_err(|error| {
        //
        tracing::error!(
            operation = "serve_http",
            sdk_err = ?error,
            "Axum SDK server error",
        );
    })
    .with_context(|| "server error")
}

// Installs Ctrl+C and (on unix) SIGTERM handlers that stop the server.
async fn shutdown_signal() {
    //
    let ctrl_c = async {
        //
        let Some(failure) = signal::ctrl_c().await.err() else {
            return;
        };

        tracing::error!(
            operation = "install_ctrl_c_handler",
            sdk_err = %failure,
            "Tokio signal handler installation failed",
        );

        pending::<()>().await;
    };

    #[cfg(unix)]
    {
        let terminate = async {
            //
            let mut terminate_recv = match signal::unix::signal(
                signal::unix::SignalKind::terminate(),
            ) {
                //
                Ok(terminate_recv) => terminate_recv,

                Err(err) => {
                    //
                    tracing::error!(
                        operation = "install_sigterm_handler",
                        sdk_err = ?err,
                        "Tokio signal handler installation failed",
                    );

                    pending::<()>().await;

                    return;
                }
            };

            terminate_recv.recv().await;
        };

        tokio::select! {
            //
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
