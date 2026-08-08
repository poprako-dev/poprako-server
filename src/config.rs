//! Application configuration loaded from a JSON file at startup.

use anyhow::Context as _;
use serde::Deserialize;

/// Runtime configuration for the HTTP server.
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    /// IP address or hostname the HTTP server binds to.
    pub http_host: String,
    /// TCP port the HTTP server listens on.
    pub http_port: u16,
}

impl AppConfig {
    /// Reads the application configuration from `application_config.json` in the
    /// current working directory.
    pub async fn from_default_file() -> anyhow::Result<Self> {
        //
        let content = tokio::fs::read_to_string("application_config.json")
            .await
            .inspect_err(|error| {
                //
                tracing::error!(
                    operation = "read_application_config",
                    sdk_err = ?error,
                    "Tokio SDK file read error",
                );
            })
            .with_context(
                || "[ApplicationConfig::from_default_file] Failed to read application_config.json",
            )?;

        serde_json::from_str(&content)
            .inspect_err(|error| {
                //
                tracing::error!(
                    operation = "parse_application_config",
                    sdk_err = ?error,
                    "JSON SDK deserialization error",
                );
            })
            .with_context(
                || "[ApplicationConfig::from_default_file] Failed to parse application_config.json",
            )
    }
}
