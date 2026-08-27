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
use std::num::NonZeroUsize;
use std::sync::Arc;

use anyhow::Context as _;

use poprako_server::{
    AppConfig, AsyncEffectDevelop, Harn, HybNucl, HybRepo, JwtAuth,
    R2ImagePool, RdbContext, RdbCore, RdbNucl, RdbProm, ReptRead, Sched,
    Serial,
};

/// Application entry point.
///
/// Parses CLI flags, loads configuration, initializes runtime dependencies
/// (database pool, authentication, image pool, effect dispatcher), wires them
/// into an application harness, and launches the HTTP server. Pass `--swagger`
/// to print the `OpenAPI` spec to stdout instead of launching the server.
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

    let http_addr = ToSocketAddrs::to_socket_addrs(&format!(
        "{}:{}",
        config.http.host, config.http.port
    ))
    .into_iter()
    .find_map(|mut addrs| addrs.next())
    .context("no address resolved for HTTP listen address")?;

    let core = RdbCore::from_env()?;

    let nucl = HybNucl::new(
        RdbNucl::<ReptRead>::new(core.clone()),
        RdbNucl::<Serial>::new(core.clone()),
    );

    let repo = HybRepo::new(core.clone());

    let (auth, image_pool) = (JwtAuth::from_env()?, R2ImagePool::from_env()?);

    let develop = AsyncEffectDevelop::new::<RdbContext<ReptRead>, _>(
        Arc::new(HybRepo::new(core.clone())),
        NonZeroUsize::new(1024).context("buf_size cannot be 0")?,
    );

    let (prom, sched) = (
        RdbProm::new(core.clone(), image_pool.clone(), develop.clone()),
        Sched::new(core.clone()),
    );

    let harn = Harn::new(config, nucl, repo, prom, auth, image_pool, develop);

    let serve_rest = poprako_server::serve(harn.clone(), http_addr).await;

    tokio::join!(harn.prom().close(), harn.develop().close(), sched.close());

    serve_rest
}
