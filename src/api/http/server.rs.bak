// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use std::fmt::Debug;
// use std::net::SocketAddr;
// 
// use anyhow::Context as _;
// use tokio::net::{TcpListener, ToSocketAddrs};
// use tokio::signal;
// 
// use crate::api::http::router;
// use crate::harness::Harness;
// 
// async fn shutdown_signal() {
//     let ctrl_c = async {
//         signal::ctrl_c()
//             .await
//             .expect("failed to install Ctrl+C handler");
//     };
// 
//     #[cfg(unix)]
//     if cfg!(unix) {
//         let terminate = async {
//             signal::unix::signal(signal::unix::SignalKind::terminate())
//                 .expect("failed to install SIGTERM handler")
//                 .recv()
//                 .await;
//         };
// 
//         tokio::select! {
//             () = ctrl_c => {},
//             () = terminate => {},
//         }
//     } else {
//         ctrl_c.await;
//     }
// 
//     tracing::info!(
//         "[server::shutdown_signal] shutdown signal received, starting graceful shutdown"
//     );
// }
// 
// pub async fn serve<A>(harn: Harness, addr: A) -> anyhow::Result<()>
// where
//     A: ToSocketAddrs + Debug,
// {
//     let listener = TcpListener::bind(&addr)
//         .await
//         .with_context(|| format!("[server::serve] failed to bind listener on {:?}", addr))?;
// 
//     tracing::info!(addr = ?addr, "[server::serve] listening");
// 
//     let app = router::new(harn.clone()).with_state(harn);
// 
//     tracing::info!(
//         "[server::serve] server is ready to accept connections on {:?}",
//         listener.local_addr()?
//     );
// 
//     axum::serve(
//         listener,
//         app.into_make_service_with_connect_info::<SocketAddr>(),
//     )
//     .with_graceful_shutdown(shutdown_signal())
//     .await
//     .with_context(|| "[server::serve] server error")
// }
