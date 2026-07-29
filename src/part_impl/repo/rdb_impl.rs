//! Diesel-backed repository adapter.

use crate::part_impl::shared::RdbCore;

#[macro_use]
mod submit_query;
/// Announcement repository operations.
pub mod announcement;
/// Assignment repository operations.
pub mod assignment;
/// Assignment invitation repository operations.
pub mod assignment_invitation;
/// Chapter repository operations.
pub mod chapter;
/// Comic repository operations.
pub mod comic;
/// Immutable comic archive repository operations.
pub mod comic_archive;
/// Comment repository operations.
pub mod comment;
/// Entity types for the RDB repository.
pub mod entity;
/// Batch include helpers.
pub mod incl;
/// Member repository operations.
pub mod member;
/// Member invitation repository operations.
pub mod member_invitation;
/// Page repository operations.
pub mod page;
/// Diesel-generated schema.
pub mod schema;
/// System mail repository operations.
pub mod system_mail;
/// Team repository operations.
pub mod team;
/// Term repository operations.
pub mod term;
/// Termbase repository operations.
pub mod termbase;
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod test_shared;
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
mod tests;
/// Unit repository operations.
pub mod unit;
/// User repository operations.
pub mod user;
/// Workset repository operations.
pub mod workset;

/// Diesel-backed repository handle wrapping a connection pool.
#[derive(Clone)]
pub struct RdbRepo {
    /// Shared database connection pool and repository state.
    core: RdbCore,
}

impl RdbRepo {
    /// Builds a new `RdbRepo` from an [`RdbCore`] connection pool.
    pub fn new(core: RdbCore) -> Self {
        Self { core }
    }
}
