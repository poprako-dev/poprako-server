//! Concrete production [`Harn`] type used as the axum application state.
//!
//! Handlers depend on the concrete port bundle rather than a generic `Harn`
//! so that axum state extraction stays simple. The underlying usecase
//! functions remain generic; they are monomorphized over these concrete
//! adapters at the handler call sites.

use crate::harn::Harn;
use crate::part_impl::RdbContext;
use crate::part_impl::auth_jwt::JwtAuth;
use crate::part_impl::drive_rdb::RdbDrive;
use crate::part_impl::effect_async::AsyncEffectDevelop;
use crate::part_impl::image_r2::R2ImagePool;
use crate::part_impl::prom_rdb::RdbProm;
use crate::part_impl::repo_rdb::RdbRepo;

/// Production harness type backing the HTTP server state.
pub type AppHarn = Harn<
    RdbContext,
    RdbDrive,
    RdbRepo,
    RdbProm,
    JwtAuth,
    R2ImagePool,
    AsyncEffectDevelop<RdbContext, RdbRepo>,
>;
