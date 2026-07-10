//! Diesel-backed repository adapter.

use async_trait::async_trait;

use crate::util::DeriveTransactional;

use crate::part_impl::shared::RdbCore;

/// Allocates a connection and calls a free query function.
///
/// The function must accept `(&mut AsyncPgConnection, args...)` and return
/// a `Future<Output = Result<T, crate::result::Error>>`.
macro_rules! submit_query {
    ($core:expr, $fn:path $(, $arg:expr)* $(,)?) => {{
        let mut conn = $core.get().await?;
        $fn(&mut *conn, $($arg),*).await
    }};
}

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

// FIXME: forbidden pattern.
use self::schema as generated;

pub(crate) use generated::{
    t_archived_chapter, t_archived_comic, t_archived_translation, t_assignment,
    t_assignment_invitation, t_chapter, t_comic, t_page, t_unit, t_user,
    t_workset,
};

/// System mail repository operations.
pub mod system_mail;
/// Team repository operations.
pub mod team;
/// Unit repository operations.
pub mod unit;
/// User repository operations.
pub mod user;
/// Workset repository operations.
pub mod workset;

#[cfg(all(test, feature = "repo"))]
mod test_shared;

/// Diesel-backed repository handle wrapping a connection pool for non-transactional operations.
pub struct RdbRepo {
    core: RdbCore,
}

impl RdbRepo {
    pub fn new(core: RdbCore) -> Self {
        Self { core }
    }
}

/// Zero-sized marker type representing a live transaction handle for Diesel-backed repositories.
pub struct RdbRepoTransactional;

#[async_trait]
impl DeriveTransactional for RdbRepo {
    type Transactional = RdbRepoTransactional;

    async fn derive_transactional(&self) -> Self::Transactional {
        RdbRepoTransactional
    }
}
