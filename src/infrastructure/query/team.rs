use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;

use tracing::Level;
use tracing::instrument;

use crate::domain::model::aggregate::team::TeamAggr;
use crate::domain::query::team::TeamQuery;
use crate::domain::result::{DomainError, DomainResult};
use crate::infrastructure::query::Query;
use crate::infrastructure::query::entity::team::TeamRow;
use crate::infrastructure::query::schema::t_team::dsl::*;
use crate::submit_query;
use crate::util::err::ErrorTrace as _;
use crate::util::i18n::trl;

#[instrument(skip(conn), level = Level::DEBUG)]
pub async fn get_by_id(conn: &mut AsyncPgConnection, id: String) -> DomainResult<TeamAggr> {
    let row: TeamRow = t_team
        .filter(f_id.eq(&id))
        .select(TeamRow::as_select())
        .first(conn)
        .await
        .optional()?
        .ok_or(DomainError::expected_argument(trl("error-team-not-found")))
        .trace_debug()?;

    Ok(row.into())
}

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait]
impl TeamQuery for Query {
    #[instrument(skip(self), level = Level::DEBUG)]
    async fn get_by_id(&self, id: String) -> DomainResult<TeamAggr> {
        submit_query!(self.pool, get_by_id, id)
    }
}
