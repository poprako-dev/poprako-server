//! Concrete production [`Harn`] type used as the axum application state.
//!
//! Handlers depend on the concrete port bundle rather than a generic `Harn`
//! so that axum state extraction stays simple. The underlying usecase
//! functions remain generic; they are monomorphized over these concrete
//! adapters at the handler call sites.

use crate::harn::{Harn, NuclProxy};
use crate::part::nucl::{RepeatableRead, Serializable};
use crate::part_impl::auth::jwt_impl::JwtAuth;
use crate::part_impl::effect::async_impl::AsyncEffectDevelop;
use crate::part_impl::image::r2_impl::R2ImagePool;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::prom::rdb_impl::RdbProm;
use crate::part_impl::repo::HybRepo;

/// Production harness type backing the HTTP server state.
pub type AppHarn = Harn<
    NuclProxy<RdbNucl<RepeatableRead>, RdbNucl<Serializable>>,
    HybRepo,
    RdbProm,
    JwtAuth,
    R2ImagePool,
    AsyncEffectDevelop,
>;
