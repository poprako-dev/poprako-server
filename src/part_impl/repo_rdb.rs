//! Diesel-backed repository adapter.

use async_trait::async_trait;

use crate::util::DeriveTransactional;

use super::repo_rdb_shared;
use super::repo_rdb_shared::RdbShared;

pub mod entity;

#[path = "../infra/repo/schema.rs"]
pub mod schema;

pub struct RdbRepo {
    shared: RdbShared,
}

impl RdbRepo {
    pub fn new(shared: RdbShared) -> Self {
        Self { shared }
    }

    pub(super) async fn conn(
        &self,
        location: &'static str,
    ) -> Result<repo_rdb_shared::RdbConn, crate::result::RootError> {
        self.shared.conn(location).await
    }
}

pub struct RdbRepoTransactional;

#[async_trait]
impl DeriveTransactional for RdbRepo {
    type Transactional = RdbRepoTransactional;

    async fn transactional(&self) -> Self::Transactional {
        RdbRepoTransactional
    }
}
