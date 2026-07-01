//! Diesel-backed repository adapter.

use async_trait::async_trait;

use crate::util::DeriveTransactional;

use super::shared_rdb::{RdbConn, RdbShared};
use crate::result::RootError;

pub mod entity;

mod schema;

pub struct RdbRepo {
    shared: RdbShared,
}

impl RdbRepo {
    pub fn new(shared: RdbShared) -> Self {
        Self { shared }
    }

    pub async fn conn(&self) -> Result<RdbConn, RootError> {
        self.shared.conn().await
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
