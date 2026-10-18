//! HTTP server configuration.

use std::num::NonZeroU32;
use std::time::Duration;

use anyhow::{Context as _, ensure};
use axum::http::HeaderValue;
use serde::Deserialize;
use url::Url;

/// A validated, serialized HTTP or HTTPS origin for exact CORS matching.
#[derive(Clone, Deserialize)]
#[serde(try_from = "String")]
pub struct CorsOrigin(HeaderValue);

impl CorsOrigin {
    /// Returns the already-validated origin header.
    #[must_use]
    pub fn header(&self) -> HeaderValue {
        self.0.clone()
    }
}

impl Default for CorsOrigin {
    // Supplies the default runtime policy.
    fn default() -> Self {
        Self(HeaderValue::from_static("https://poprako.com"))
    }
}

impl TryFrom<String> for CorsOrigin {
    // Reports invalid origin syntax or header values.
    type Error = anyhow::Error;

    // Validates the serialized origin before constructing its header.
    fn try_from(value: String) -> Result<Self, Self::Error> {
        //
        let url = Url::parse(&value)
            .context("http.cors_origin must be an HTTP(S) origin")?;

        ensure!(
            matches!(url.scheme(), "http" | "https")
                && url.origin().ascii_serialization() == value,
            "http.cors_origin must be a canonical HTTP(S) origin without credentials, path, query or fragment"
        );

        let header = HeaderValue::try_from(value)
            .context("http.cors_origin must be a valid header value")?;

        Ok(Self(header))
    }
}

/// Shared request quota for one application instance.
#[derive(Clone, Copy, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RateLimitConfig {
    /// Sustained number of requests accepted per second.
    pub requests_per_second: NonZeroU32,
    /// Maximum number of requests accepted in one burst.
    pub burst: NonZeroU32,
}

impl RateLimitConfig {
    /// Rejects rates that cannot be represented by the limiter's clock.
    ///
    /// # Errors
    /// Returns an error when the rate requires subnanosecond replenishment.
    pub fn validate(self) -> anyhow::Result<()> {
        //
        ensure!(
            u128::from(self.requests_per_second.get())
                <= Duration::from_secs(1).as_nanos(),
            "http.rate_limit.requests_per_second exceeds nanosecond resolution"
        );

        Ok(())
    }
}

impl Default for RateLimitConfig {
    // Supplies the default runtime policy.
    fn default() -> Self {
        Self {
            requests_per_second: NonZeroU32::MIN.saturating_add(19),
            burst: NonZeroU32::MIN.saturating_add(79),
        }
    }
}

/// Runtime settings for the HTTP server.
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HttpConfig {
    /// IP address or hostname the HTTP server binds to.
    pub host: String,

    /// TCP port the HTTP server listens on.
    pub port: u16,

    /// Single browser origin allowed to access the API.
    pub cors_origin: CorsOrigin,
    /// Shared quota applied across this application's HTTP routes.
    pub rate_limit: RateLimitConfig,
}

impl Default for HttpConfig {
    // Supplies the default runtime policy.
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8888,
            cors_origin: CorsOrigin::default(),
            rate_limit: RateLimitConfig::default(),
        }
    }
}
