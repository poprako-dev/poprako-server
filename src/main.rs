#![recursion_limit = "256"]
#![deny(unsafe_code)]
#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::complexity)]
#![deny(clippy::perf)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
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

use std::net::ToSocketAddrs;
use std::sync::Arc;

use anyhow::Context as _;

use poprako_server::{
    AppConfig, AsyncEffectDevelop, Harn, HybRepo, JwtAuth, R2ImagePool,
    RdbCore, RdbNucl, RdbProm, Sched,
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
    poprako_server::init_log();

    if let Err(err) = dotenvy::dotenv() {
        //
        tracing::warn!(
            operation = "load_dotenv",
            sdk_err = ?err,
            ".env loading failed; continuing with process environment",
        );
    }

    let config = AppConfig::from_default_file()
        .await
        .context("failed to load application configuration")?;

    let core = RdbCore::from_env()?;

    let (nucl, repo) = (RdbNucl::new(core.clone()), HybRepo::new(core.clone()));

    let (auth, image_pool) = (JwtAuth::from_env()?, R2ImagePool::from_env()?);

    let develop =
        AsyncEffectDevelop::new(Arc::new(HybRepo::new(core.clone())), 1024);

    let (prom, sched) = (
        RdbProm::new(core.clone(), image_pool.clone(), develop.clone()),
        Sched::new(core.clone()),
    );

    let harn = Harn::new(nucl, repo, prom, auth, image_pool, develop);

    let http_addr = ToSocketAddrs::to_socket_addrs(&format!(
        "{}:{}",
        config.http_host, config.http_port
    ))
    .into_iter()
    .find_map(|mut addrs| addrs.next())
    .context("no address resolved for HTTP listen address")?;

    let serve_rest = poprako_server::serve(harn.clone(), http_addr).await;

    tokio::join!(harn.prom().close(), harn.develop().close(), sched.close());

    serve_rest
}
