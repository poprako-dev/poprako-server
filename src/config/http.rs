//! HTTP server configuration.

use serde::Deserialize;

/// Runtime settings for the HTTP server.
#[derive(Debug, Deserialize)]
pub struct HttpConfig {
    /// IP address or hostname the HTTP server binds to.
    pub host: String,

    /// TCP port the HTTP server listens on.
    pub port: u16,
}
