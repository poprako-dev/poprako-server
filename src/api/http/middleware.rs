//! HTTP middleware: authorization token verification and request latency
//! logging.

/// Authorization token middleware.
pub mod auth;
/// Cross-origin API access middleware.
pub mod cors;
/// Sliding-window metrics middleware helper module.
pub mod metric;
/// Rate limiting middleware.
pub mod rate_limit;
/// Request ID and tracing middleware.
pub mod trace;

// Custom request latency logging is disabled in favor of `TraceLayer`.
// pub mod latency;
