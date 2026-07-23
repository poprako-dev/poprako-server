#![recursion_limit = "256"]
#![deny(unsafe_code)]
#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::complexity)]
#![deny(clippy::perf)]
#![deny(clippy::unwrap_used)]
// #![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]
#![deny(clippy::exit)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::mod_module_files)]
#![warn(clippy::style)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

#[cfg(feature = "swagger")]
use std::io::Write as _;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use anyhow::Context as _;
#[cfg(feature = "swagger")]
use utoipa::OpenApi as _;

use poprako_server::{
    AppConfig, AppHarn, AsyncEffectDevelop, Harn, JwtAuth, R2ImagePool,
    RdbCore, RdbDrive, RdbProm, RdbRepo, RdbSched, init_log, serve,
};

/// Application entry point.
///
/// Parses CLI flags, loads configuration, initializes runtime dependencies
/// (database pool, authentication, image pool, effect dispatcher), wires them
/// into an application harness, and starts the HTTP server. Pass `--swagger`
/// to print the `OpenAPI` spec to stdout instead of starting the server.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //
    dotenvy::dotenv().expect(".env file should be valid");

    init_log();

    let config = AppConfig::from_default_file()
        .await
        .context("failed to load application configuration")?;

    let core = RdbCore::from_env()?;

    let drive = RdbDrive::new(core.clone());

    let repo = RdbRepo::new(core.clone());

    let repo_effect = Arc::new(RdbRepo::new(core.clone()));

    let auth = JwtAuth::from_env()?;

    let image_pool = R2ImagePool::from_env()?;

    let prom = RdbProm::new(core.clone(), image_pool.clone());

    let sched = RdbSched::new(core.clone());

    let develop = AsyncEffectDevelop::new(repo_effect, 1024);

    let harn: AppHarn = Harn::new(drive, repo, prom, auth, image_pool, develop);

    let http_addr: SocketAddr = ToSocketAddrs::to_socket_addrs(&format!(
        "{}:{}",
        config.http_host, config.http_port
    ))
    .context("failed to resolve HTTP listen address")?
    .next()
    .context("no address resolved for HTTP listen address")?;

    let serve_outcome = serve(harn.clone(), http_addr).await;

    harn.develop().close().await;

    harn.prom().close().await;

    sched.close().await;

    serve_outcome
}
