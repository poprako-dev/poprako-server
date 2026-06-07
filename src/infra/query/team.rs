use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use tracing::Level;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::domain::model::aggr::team::{TeamAggr, TeamForm, TeamUpdate};
use crate::domain::query::team::TeamQuery;
use crate::domain::query::team::TeamQueryTransactional;
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::RdbQuery;
use crate::infra::query::RdbQueryTransactional;
use crate::infra::query::entity::team::TeamAspect;
use crate::infra::query::entity::team::TeamEntry;
use crate::infra::query::entity::team::TeamRow;
use crate::infra::query::schema::t_team::dsl::*;
use crate::submit_query;

// ── Free functions ─────────────────────────────────────────────────────────

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn get_by_id(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<TeamAggr> {
    let row: TeamRow = t_team
        .filter(f_id.eq(&id))
        .select(TeamRow::as_select())
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| DomainError::expected_argument(trl("error-team-not-found")))?;

    Ok(row.into())
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn list(
    conn: &mut AsyncPgConnection,
    offset: i64,
    limit: i64,
) -> DomainResult<Vec<TeamAggr>> {
    let rows: Vec<TeamRow> = t_team
        .order(f_created_at.desc())
        .offset(offset)
        .limit(limit)
        .select(TeamRow::as_select())
        .load(conn)
        .await?;

    let result: Vec<TeamAggr> = rows.into_iter().map(|r| r.into()).collect();

    Ok(result)
}

#[instrument(err, skip(conn, form), level = Level::DEBUG)]
pub async fn create(conn: &mut AsyncPgConnection, form: &TeamForm) -> DomainResult<TeamAggr> {
    let now = OffsetDateTime::now_utc();

    let entry = TeamEntry {
        f_id: &form.id,
        f_name: &form.name,
        f_description: &form.description,
        f_workset_next_index: 0,
        f_created_at: now,
        f_updated_at: now,
    };

    diesel::insert_into(t_team)
        .values(&entry)
        .execute(conn)
        .await?;

    let row: TeamRow = t_team
        .filter(f_id.eq(&entry.f_id))
        .select(TeamRow::as_select())
        .first(conn)
        .await?;

    Ok(row.into())
}

#[instrument(err, skip(conn, input), level = Level::DEBUG)]
pub async fn update(conn: &mut AsyncPgConnection, input: &TeamUpdate) -> DomainResult<()> {
    let now = OffsetDateTime::now_utc();

    let changes = TeamAspect::new(now)
        .name(&input.name)
        .description(&input.description);

    let affected = diesel::update(t_team.filter(f_id.eq(&input.id)))
        .set(&changes)
        .execute(conn)
        .await?;

    if affected == 0 {
        return Err(DomainError::expected_argument(trl("error-team-not-found")));
    }

    Ok(())
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn delete(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<()> {
    let affected = diesel::delete(t_team.filter(f_id.eq(id)))
        .execute(conn)
        .await?;

    if affected == 0 {
        return Err(DomainError::expected_argument(trl("error-team-not-found")));
    }

    Ok(())
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn prefill_avatar_key(
    conn: &mut AsyncPgConnection,
    id: &str,
    key: &str,
) -> DomainResult<()> {
    let now = OffsetDateTime::now_utc();

    let changes = TeamAspect::new(now).avatar_key(key).avatar_uploaded(false);

    let affected = diesel::update(t_team.filter(f_id.eq(id)))
        .set(&changes)
        .execute(conn)
        .await?;

    if affected == 0 {
        return Err(DomainError::expected_argument(trl("error-team-not-found")));
    }

    Ok(())
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn mark_avatar_uploaded(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<()> {
    let now = OffsetDateTime::now_utc();

    let changes = TeamAspect::new(now).avatar_uploaded(true);

    let affected = diesel::update(t_team.filter(f_id.eq(id)))
        .set(&changes)
        .execute(conn)
        .await?;

    if affected == 0 {
        return Err(DomainError::expected_argument(trl("error-team-not-found")));
    }

    Ok(())
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn increment_workset_next_index(
    conn: &mut AsyncPgConnection,
    id: &str,
) -> DomainResult<i32> {
    let affected = diesel::update(t_team.filter(f_id.eq(id)))
        .set(f_workset_next_index.eq(f_workset_next_index + 1))
        .execute(conn)
        .await?;

    if affected == 0 {
        return Err(DomainError::expected_argument(trl("error-team-not-found")));
    }

    let new_value: i32 = t_team
        .filter(f_id.eq(id))
        .select(f_workset_next_index)
        .first(conn)
        .await?;

    // The column now holds the incremented value; subtract 1 to get the allocated index.
    Ok(new_value - 1)
}

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait]
impl TeamQuery for RdbQuery {
    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn get_by_id(&self, id: &str) -> DomainResult<TeamAggr> {
        submit_query!(self.pool, get_by_id, id)
    }

    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn list(&self, offset: i64, limit: i64) -> DomainResult<Vec<TeamAggr>> {
        submit_query!(self.pool, list, offset, limit)
    }

    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn prefill_avatar_key(&self, id: &str, key: &str) -> DomainResult<()> {
        submit_query!(self.pool, prefill_avatar_key, id, key)
    }

    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn mark_avatar_uploaded(&self, id: &str) -> DomainResult<()> {
        submit_query!(self.pool, mark_avatar_uploaded, id)
    }
}

#[async_trait]
impl<'c> TeamQueryTransactional for RdbQueryTransactional<'c> {
    async fn create(&mut self, form: &TeamForm) -> DomainResult<TeamAggr> {
        create(self.conn, form).await
    }

    async fn update(&mut self, input: &TeamUpdate) -> DomainResult<()> {
        update(self.conn, input).await
    }

    async fn delete(&mut self, id: &str) -> DomainResult<()> {
        delete(self.conn, id).await
    }

    async fn increment_workset_next_index(&mut self, id: &str) -> DomainResult<i32> {
        increment_workset_next_index(self.conn, id).await
    }
}
