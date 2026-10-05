#![recursion_limit = "256"]

//! Crate root: explicit public re-exports and internal module organization for
//! the `PopRaKo` application core.

// HTTP API layer (handlers, middleware, server, router, OpenAPI).
mod api;
// Core business-logic helpers that coordinate domain rules across models.
mod complex;
// Application configuration parsing and access.
mod config;
// Inbound request and outbound response DTOs for the HTTP API layer.
mod data;
// Long-lived production background jobs.
mod extra;
// Application harness wiring all ports together for production and test use.
mod harn;
// Tracing-subscriber initialisation shared across binaries.
mod log;
// Persisted business entity model definitions backed by database tables.
mod model;
// Port trait definitions (repo, auth, image, prom, effect) for the application
// core.
mod part;
// Root error and result types used across all layers.
mod result;
// Shared RDB infrastructure used by port implementations.
mod shared;

#[cfg(test)]
// Internal tests utility helpers for fixtures and assertions.
mod test_util;

// Application use cases orchestrating the ports-and-transaction-steps core.
mod usecase;
// Shared utility functions (snowflake ID generation, etc.).
mod util;
// Domain value types, enums, and small typed concepts shared by models and use
// cases.
mod value;

// Concrete port implementations: repo, auth, prom, image, effect, nucl.
/// Concrete application port adapters.
pub mod part_impl;

/// Benchmark entry points.
#[cfg(feature = "benchmark")]
#[doc(hidden)]
pub mod benchmark;

pub use poprako_rdb_core::RdbCore;

#[cfg(feature = "swagger")]
pub use crate::api::http::openapi::ApiDoc;

#[cfg(feature = "benchmark")]
pub use crate::complex::user as user_complex;

pub use crate::api::http::server::serve;
pub use crate::api::http::state::AppHarn;
pub use crate::config::AppConfig;
pub use crate::config::http::HttpConfig;
pub use crate::config::image::ImageConfig;
pub use crate::config::sched::{SchedConfig, SchedTaskConfig};
pub use crate::extra::sched::task::{SchedNext, SchedTask, SchedTaskRunner};
pub use crate::extra::sched::{Sched, SchedDesc};
pub use crate::extra::subtree_delete::SubtreeDeleteTask;
pub use crate::harn::Harn;
pub use crate::log::init_log;
pub use crate::part::nucl::{ReptRead, Serial};
pub use crate::part::prom::payload::PromPayload;
pub use crate::part_impl::auth::jwt_impl::JwtAuth;
pub use crate::part_impl::effect::async_impl::AsyncEffectDevelop;
pub use crate::part_impl::effect::async_impl::actor::{
    EffectActor, EffectActorDesc,
};
pub use crate::part_impl::nucl::rdb_impl::{HybNucl, RdbNucl};
pub use crate::part_impl::obj_dept::r2_impl::R2ObjDeptPool;
pub use crate::part_impl::obj_dept::{NormObjDept, RdbObjDeptProm};
pub use crate::part_impl::prom::dispatch::dispatch as dispatch_prom;
pub use crate::part_impl::repo::HybRepo;
pub use crate::result::{BaseError, BaseRest};
pub use crate::shared::RdbContext;
