//! Diesel-backed repository adapter.

use async_trait::async_trait;

use crate::util::DeriveTransactional;

use super::shared_rdb::RdbShared;

/// Allocates a connection and calls a free query function.
///
/// The function must accept `(&mut AsyncPgConnection, args...)` and return
/// a `Future<Output = Result<T, crate::result::Error>>`.
macro_rules! submit_query {
    ($shared:expr, $fn:path $(, $arg:expr)* $(,)?) => {{
        let mut conn = $shared.get().await?;
        $fn(&mut *conn, $($arg),*).await
    }};
}

pub mod comic;
pub mod entity;
pub mod incl;
pub mod member;
pub mod member_invitation;
pub mod schema;
pub mod system_mail;
pub mod team;
pub mod user;
pub mod workset;

pub struct RdbRepo {
    shared: RdbShared,
}

impl RdbRepo {
    pub fn new(shared: RdbShared) -> Self {
        Self { shared }
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
