//! Application configuration and runtime policies.

// Integer-second decoding shared by runtime duration policies.
mod duration;

/// Chapter artwork upload configuration.
pub mod artwork;
/// Process-local cache retention policies.
pub mod cache;
/// Best-effort event queue configuration.
pub mod effect;
/// HTTP server configuration.
pub mod http;
/// Image upload configuration.
pub mod image;
/// Invitation expiration delays.
pub mod invitation;
/// Database repository batching policies.
pub mod repo;
/// Background task scheduling and shutdown policies.
pub mod sched;
/// Hierarchy cleanup scheduling policy.
pub mod subtree_delete;

#[cfg(test)]
mod tests;

use std::io::ErrorKind;
use std::path::Path;

use anyhow::Context as _;
use serde::Deserialize;

use crate::config::artwork::ArtworkConfig;
use crate::config::cache::CacheConfig;
use crate::config::effect::EffectConfig;
use crate::config::http::HttpConfig;
use crate::config::image::ImageConfig;
use crate::config::invitation::InvitationConfig;
use crate::config::repo::RepoConfig;
use crate::config::sched::SchedConfig;
use crate::config::subtree_delete::SubtreeDeleteConfig;

// Runtime configuration file loaded from the current working directory.
const DEFAULT_FILE_NAME: &str = "app_config.toml";

/// Runtime application configuration.
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    /// HTTP server configuration.
    pub http: HttpConfig,

    /// Image upload configuration.
    pub image: ImageConfig,

    /// Chapter artwork upload limits.
    pub artwork: ArtworkConfig,

    /// Process-local cache memory and lease budgets.
    pub cache: CacheConfig,
    /// Database query and cleanup batch sizes.
    pub repo: RepoConfig,

    /// Expiration delays for newly created invitations.
    pub invitation: InvitationConfig,

    /// Best-effort event queue capacity.
    pub effect: EffectConfig,
    /// Shared background scheduler shutdown policy.
    pub sched: SchedConfig,
    /// Hierarchy cleanup scheduling policy.
    pub subtree_delete: SubtreeDeleteConfig,
}

impl AppConfig {
    /// Reads the application configuration from `app_config.toml` in the
    /// current working directory, or uses defaults when the file is absent.
    ///
    /// # Errors
    ///
    /// Returns an error for other read failures, invalid TOML or invalid values.
    pub async fn from_default_file() -> anyhow::Result<Self> {
        Self::from_file(Path::new(DEFAULT_FILE_NAME)).await
    }

    // Loads an optional file without changing the process working directory.
    async fn from_file(path: &Path) -> anyhow::Result<Self> {
        //
        let content = match tokio::fs::read_to_string(path).await {
            //
            Ok(content) => content,

            Err(error) if error.kind() == ErrorKind::NotFound => String::new(),

            Err(error) => {
                //
                tracing::error!(
                    operation = "read_application_config",
                    sdk_error = ?error,
                    "Tokio SDK file read error",
                );

                return Err(error)
                    .context("failed to read application configuration");
            }
        };

        Self::parse(&content)
            .inspect_err(|error| {
                //
                tracing::error!(
                    operation = "parse_application_config",
                    config_error = ?error,
                    "application configuration parsing or validation failed",
                );
            })
            .with_context(
                || "[ApplicationConfig::from_default_file] Failed to parse app_config.toml",
            )
    }

    // Parses and validates runtime application configuration.
    fn parse(content: &str) -> anyhow::Result<Self> {
        //
        let config = toml::from_str::<Self>(content)?;

        config.http.rate_limit.validate()?;

        config.image.validate()?;

        config.artwork.validate()?;

        config.cache.validate()?;

        config.invitation.validate()?;

        config.effect.validate()?;

        config.subtree_delete.validate()?;

        Ok(config)
    }
}
