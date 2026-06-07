use std::net::{SocketAddr, ToSocketAddrs};

use poprako_r::api::http::server::serve;
use poprako_r::config::AppConfig;
use poprako_r::harness::Harness;
use poprako_r::infra::external::image_pool::OssImagePool;
use poprako_r::infra::external::token::JwtIssuer;
use poprako_r::infra::query::RdbQuery;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    use tracing_subscriber::layer::SubscriberExt as _;
    use tracing_subscriber::util::SubscriberInitExt as _;

    #[cfg(debug_assertions)]
    {
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
            .with(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    }
    #[cfg(not(debug_assertions))]
    {
        use tracing_subscriber::filter::LevelFilter;

        let log_folder = std::path::Path::new("logs");
        std::fs::create_dir_all(log_folder).expect("Failed to create log folder");

        let file_appender = tracing_appender::rolling::daily(log_folder, "poprako_r.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_writer(non_blocking)
                    .with_ansi(false),
            )
            .with(tracing_subscriber::EnvFilter::new(
                LevelFilter::INFO.to_string(),
            ))
            .init();

        // Keep the guard alive for the entire program lifetime.
        std::mem::forget(_guard);
    }

    let config = AppConfig::from_default_file()
        .await
        .expect("Failed to load application configuration");

    let http_addr: SocketAddr =
        ToSocketAddrs::to_socket_addrs(&format!("{}:{}", config.http_host, config.http_port))
            .expect("Failed to resolve HTTP listen address")
            .next()
            .expect("No address resolved for HTTP listen address");

    let query = RdbQuery::from_env()
        .await
        .expect("Failed to initialize query");

    let jwt_codec = JwtIssuer::from_env().expect("Failed to initialize JWT codec");

    let image_pool = OssImagePool::from_env_r2().expect("Failed to initialize image pool");

    let harn = Harness::new(query, jwt_codec, image_pool);

    serve(harn, http_addr)
        .await
        .expect("Failed to start HTTP server");
}
