//! Crate root: explicit public re-exports and internal module organization for
//! the PopRaKo application core.

/// Application configuration parsing and access.
mod config;

pub use api::http::openapi::ApiDoc;
pub use api::http::server::serve;
pub use api::http::state::AppHarn;
pub use config::AppConfig;
pub use harn::Harn;
pub use part_impl::RdbCore;
pub use part_impl::auth_jwt::JwtAuth;
pub use part_impl::drive_rdb::RdbDrive;
pub use part_impl::effect_async::AsyncEffectDevelop;
pub use part_impl::image_r2::R2ImagePool;
pub use part_impl::prom_rdb::RdbProm;
pub use part_impl::repo_rdb::RdbRepo;

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
