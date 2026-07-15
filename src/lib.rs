//! Crate root: explicit public re-exports and internal module organization for
//! the PopRaKo application core.

/// Application configuration parsing and access.
mod config;

#[cfg(feature = "swagger-ui")]
pub use api::http::openapi::ApiDoc;
pub use api::http::server::serve;
pub use api::http::state::AppHarn;
pub use complex::user::UserComplex;
pub use config::AppConfig;
pub use harn::Harn;
pub use part_impl::auth::jwt_impl::JwtAuth;
pub use part_impl::drive::rdb_impl::RdbDrive;
pub use part_impl::effect::async_impl::AsyncEffectDevelop;
pub use part_impl::image::r2_impl::R2ImagePool;
pub use part_impl::prom::rdb_impl::RdbProm;
pub use part_impl::repo::rdb_impl::RdbRepo;
pub use part_impl::shared::RdbCore;

/// Core business-logic helpers that coordinate domain rules across models.
mod complex;
/// Inbound request and outbound response DTOs for the HTTP API layer.
mod data;
/// Application harness wiring all ports together for production and test use.
mod harn;
/// Persisted business entity model definitions backed by database tables.
mod model;
/// Port trait definitions (repo, auth, image, prom, effect) for the application
/// core.
mod part;
/// Concrete port implementations: repo, auth, prom, image, effect, drive.
mod part_impl;
/// Root error and result types used across all layers.
mod result;
/// Application use cases orchestrating the ports-and-transaction-steps core.
mod usecase;
/// Domain value types, enums, and small typed concepts shared by models and use
/// cases.
mod value;

/// Shared utility functions (snowflake ID generation, etc.).
mod util;

/// HTTP API layer (handlers, middleware, server, router, OpenAPI).
mod api;

#[cfg(test)]
mod test_util;
