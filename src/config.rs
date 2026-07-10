//! Application configuration loaded from a JSON file at startup.

use anyhow::Context as _;
use serde::Deserialize;

/// Runtime configuration for the HTTP server.
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub http_host: String,
    pub http_port: u16,
}

impl AppConfig {
    /// Reads the application configuration from `application_config.json` in the
    /// current working directory.
    pub async fn from_default_file() -> anyhow::Result<Self> {
        //
        let content = tokio::fs::read_to_string("application_config.json")
            .await
            .with_context(
                || "[ApplicationConfig::from_default_file] Failed to read application_config.json",
            )?;

        serde_json::from_str(&content).with_context(
            || "[ApplicationConfig::from_default_file] Failed to parse application_config.json",
        )
    }
}
