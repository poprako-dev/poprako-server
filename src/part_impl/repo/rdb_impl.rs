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

pub mod announcement;

pub mod assignment;
pub mod assignment_invitation;
pub mod chapter;
pub mod comic;
pub mod comment;

pub mod entity;
pub mod incl;
pub mod member;
pub mod member_invitation;
pub mod page;
pub mod schema;
pub mod system_mail;
pub mod team;
pub mod unit;
pub mod user;
pub mod workset;

#[cfg(all(test, feature = "repo"))]
mod test_shared;

pub struct RdbRepo {
    core: RdbCore,
}

impl RdbRepo {
    pub fn new(core: RdbCore) -> Self {
        Self { core }
    }
}

pub struct RdbRepoTransactional;

#[async_trait]
impl DeriveTransactional for RdbRepo {
    type Transactional = RdbRepoTransactional;

    async fn derive_transactional(&self) -> Self::Transactional {
        RdbRepoTransactional
    }
}
