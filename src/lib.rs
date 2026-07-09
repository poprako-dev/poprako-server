//! Crate root: explicit public re-exports and internal module organization for
//! the PopRaKo application core.

/// Application configuration parsing and access.
mod config;

pub use api::http::openapi::ApiDoc;
pub use api::http::server::serve;
pub use api::http::state::AppHarn;
pub use config::AppConfig;
pub use harn::Harn;
pub use part_impl::shared::RdbCore;
pub use part_impl::auth::jwt_impl::JwtAuth;
pub use part_impl::drive::rdb_impl::RdbDrive;
pub use part_impl::effect::async_impl::AsyncEffectDevelop;
pub use part_impl::image::r2_impl::R2ImagePool;
pub use part_impl::prom::rdb_impl::RdbProm;
pub use part_impl::prom::rdb_impl::spawn_handler;
pub use part_impl::repo::rdb_impl::RdbRepo;

mod complex;
mod data;
mod harn;
mod model;
mod part;
mod part_impl;
mod result;
mod usecase;
mod value;

mod util;

mod api;

#[cfg(test)]
mod test_util;
