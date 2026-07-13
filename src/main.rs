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

#[cfg(feature = "swagger-ui")]
use std::io::Write as _;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::Path;
use std::sync::Arc;

use anyhow::Context as _;
#[cfg(feature = "swagger-ui")]
use utoipa::OpenApi as _;

use poprako_server::{
    AppConfig, AppHarn, AsyncEffectDevelop, Harn, JwtAuth, R2ImagePool,
    RdbCore, RdbDrive, RdbProm, RdbRepo, serve,
};

/// Application entry point.
///
/// Parses CLI flags, loads configuration, initializes runtime dependencies
/// (database pool, authentication, image pool, effect dispatcher, Prometheus
/// collector), wires them into an application harness, and starts the HTTP
/// server. Pass `--swagger` to print the `OpenAPI` spec to stdout instead of
/// starting the server.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //
    // CLI: --swagger to print swagger.json.
    #[cfg(feature = "swagger-ui")]
    if std::env::args().any(|a| a == "--swagger") {
        //
        #[allow(clippy::print_stdout)]
        {
            let doc = poprako_server::ApiDoc::openapi();

            let swagger_json = serde_json::to_string_pretty(&doc)?;

            std::io::stdout().write_all(swagger_json.as_bytes())?;
        }

        return Ok(());
    }

    dotenvy::dotenv().expect(".env file should be valid");

    let _log_guard = if cfg!(debug_assertions) {
        //
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
            .init();

        None
    } else {
        //
        let log_folder = Path::new("logs");

        std::fs::create_dir_all(log_folder)
            .context("failed to create log folder")?;

        let file_appender =
            tracing_appender::rolling::daily(log_folder, "poprako_server.log");

        let (non_blocking, log_guard) =
            tracing_appender::non_blocking(file_appender);

        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_writer(non_blocking)
            .with_ansi(false)
            .init();

        Some(log_guard)
    };

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

    let develop = AsyncEffectDevelop::new(repo_effect, 1024);

    let harn: AppHarn = Harn::new(drive, repo, prom, auth, image_pool, develop);

    let http_addr: SocketAddr = ToSocketAddrs::to_socket_addrs(&format!(
        "{}:{}",
        config.http_host, config.http_port
    ))
    .context("failed to resolve HTTP listen address")?
    .next()
    .context("no address resolved for HTTP listen address")?;

    serve(harn, http_addr).await
}
