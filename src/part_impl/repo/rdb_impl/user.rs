//! RDB-backed user repository — free query functions and thin trait impls.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::user::UserComplex;
use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::write::user::{
    UserAvatarReservation, UserCredsRepl, UserEntry, UserInfoRepl,
};
use crate::part::repo::oper::user::{
    CreateUser, DeleteUser, FindUserInfo, GetUserCredential, GetUserInfo,
    GetUserInfoExcluded, ReserveUserAvatar, UpdateUser,
};
use crate::part_impl::repo::rdb_impl::entity::user::{
    UserAspectRow, UserCredsRow, UserEntryRow, UserInfoRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::*;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::{diesel, next_version};
use crate::shared::{RdbConn, RdbContext};
use crate::value::image::{ImageExt, ImageHash};

// User repository impl blocks.
mod impls;
// ── Free functions ──────────────────────────────────────────────────────────

// Remove a user row from persistence.

// User free-function helpers.
mod helpers;

/// User RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;
