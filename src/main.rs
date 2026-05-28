use std::net::{SocketAddr, ToSocketAddrs};

use poprako_r::api::harness::Harness;
use poprako_r::api::http::server::serve;
use poprako_r::config::ApplicationConfig;
use poprako_r::infrastructure::external::image_pool::OssImagePool;
use poprako_r::infrastructure::external::token::JwtCodec;
use poprako_r::infrastructure::query::Query;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = ApplicationConfig::from_default_file()
        .await
        .expect("Failed to load application configuration");

    let http_addr: SocketAddr =
        ToSocketAddrs::to_socket_addrs(&format!("{}:{}", config.http_host, config.http_port))
            .expect("Failed to resolve HTTP listen address")
            .next()
            .expect("No address resolved for HTTP listen address");

    let query = Query::from_env().await.expect("Failed to initialize query");

    let jwt_codec = JwtCodec::from_env().expect("Failed to initialize JWT codec");

    let image_pool = OssImagePool::from_env_r2().expect("Failed to initialize image pool");

    let harn = Harness::new(query, jwt_codec, image_pool);

    serve(harn, http_addr)
        .await
        .expect("Failed to start HTTP server");
}
