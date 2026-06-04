use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;

use tracing::Level;
use tracing::instrument;

use crate::domain::model::aggr::team::TeamAggr;
use crate::domain::query::team::TeamQuery;
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::RdbQuery;
use crate::infra::query::entity::team::TeamRow;
use crate::infra::query::schema::t_team::dsl::*;
use crate::submit_query;
use poprako_util::i18n::trl;

#[instrument(skip(conn), level = Level::DEBUG)]
pub async fn get_by_id(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<TeamAggr> {
    let row: TeamRow = t_team
        .filter(f_id.eq(&id))
        .select(TeamRow::as_select())
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")).trace())?;

    Ok(row.into())
}

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait]
impl TeamQuery for RdbQuery {
    #[instrument(skip(self), level = Level::DEBUG)]
    async fn get_by_id(&self, id: &str) -> DomainResult<TeamAggr> {
        submit_query!(self.pool, get_by_id, id)
    }
}
