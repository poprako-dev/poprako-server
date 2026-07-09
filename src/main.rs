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
#![warn(clippy::cargo)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::multiple_crate_versions)]

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use anyhow::Context as _;
use utoipa::OpenApi as _;

use poprako_r::{
    AppConfig, AppHarn, AsyncEffectDevelop, Harn, JwtAuth, R2ImagePool,
    RdbCore, RdbDrive, RdbProm, RdbRepo, serve,
};

/// Application entry point.
///
/// Parses CLI flags, loads configuration, initializes runtime dependencies
/// (database pool, authentication, image pool, effect dispatcher, Prometheus
/// collector), wires them into an application harness, and starts the HTTP
/// server. Pass `--swagger` to print the OpenAPI spec to stdout instead of
/// starting the server.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // CLI: --swagger to print swagger.json.
    if std::env::args().any(|a| a == "--swagger") {
        #[allow(clippy::print_stdout)]
        {
            let doc = poprako_r::ApiDoc::openapi();
            println!("{}", serde_json::to_string_pretty(&doc)?);
        }

        return Ok(());
    }

    dotenvy::dotenv().expect(".env file should be valid");

    if cfg!(debug_assertions) {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
            .init();
    } else {
        // FIXME: rotating.
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    }

    let config = AppConfig::from_default_file()
        .await
        .context("failed to load application configuration")?;

    let core = RdbCore::from_env()?;

    let drive = RdbDrive::new(core.clone());

    let repo = RdbRepo::new(core.clone());
    let repo_effect = Arc::new(RdbRepo::new(core.clone()));

    let prom = RdbProm;

    let auth = JwtAuth::from_env()?;

    let image_pool = R2ImagePool::from_env()?;

    poprako_r::spawn_handler(core.clone(), image_pool.clone());

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

// LEGACY DISABLED: Do not use. This file is intentionally commented out.
// use std::net::{SocketAddr, ToSocketAddrs};
//
// use poprako_r::api::http::server::serve;
// use poprako_r::config::AppConfig;
// use poprako_r::harness::Harness;
// use poprako_r::infra::external::image_pool::OssImagePool;
// use poprako_r::infra::external::token::JwtIssuer;
// use poprako_r::infra::repo::RdbQuery;
//
// #[tokio::main]
// async fn main() {
//     dotenvy::dotenv().ok();
//
//     use tracing_subscriber::layer::SubscriberExt as _;
//     use tracing_subscriber::util::SubscriberInitExt as _;
//
//     #[cfg(debug_assertions)]
//     {
//         tracing_subscriber::registry()
//             .with(tracing_subscriber::fmt::layer())
//             .with(tracing_subscriber::EnvFilter::from_default_env())
//             .init();
//     }
//     #[cfg(not(debug_assertions))]
//     {
//         use tracing_subscriber::filter::LevelFilter;
//
//         let log_folder = std::path::Path::new("logs");
//         std::fs::create_dir_all(log_folder).expect("Failed to create log folder");
//
//         let file_appender = tracing_appender::rolling::daily(log_folder, "poprako_r.log");
//         let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
//
//         tracing_subscriber::registry()
//             .with(
//                 tracing_subscriber::fmt::layer()
//                     .json()
//                     .with_writer(non_blocking)
//                     .with_ansi(false),
//             )
//             .with(tracing_subscriber::EnvFilter::new(
//                 LevelFilter::INFO.to_string(),
//             ))
//             .init();
//
//         std::mem::forget(_guard);
//     }
//
//     let config = AppConfig::from_default_file()
//         .await
//         .expect("Failed to load application configuration");
//
//     let http_addr: SocketAddr =
//         ToSocketAddrs::to_socket_addrs(&format!("{}:{}", config.http_host, config.http_port))
//             .expect("Failed to resolve HTTP listen address")
//             .next()
//             .expect("No address resolved for HTTP listen address");
//
//     let repo = RdbQuery::from_env()
//         .await
//         .expect("Failed to initialize repo");
//
//     let jwt_codec = JwtIssuer::from_env().expect("Failed to initialize JWT codec");
//
//     let image_pool = OssImagePool::from_env_r2().expect("Failed to initialize image pool");
//
//     let harn = Harness::new(repo, jwt_codec, image_pool);
//
//     serve(harn, http_addr)
//         .await
//         .expect("Failed to start HTTP server");
// }
