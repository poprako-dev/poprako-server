use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use time::OffsetDateTime;
use tracing::{instrument, Level};

use poprako_util::page::Page;

use crate::domain::model::aggr::local_message::{
    LocalMessageAggr, LocalMessageForm, LocalMessageMark, LocalMessageStatus,
};
use crate::domain::query_legacy::local_message::{LocalMessageQuery, LocalMessageQueryTransactional};
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::entity::local_message::{
    LocalMessageAspect, LocalMessageEntry, LocalMessageRow,
};
use crate::infra::query::schema::t_local_message::dsl::*;
use crate::infra::query::{RdbQuery, RdbQueryTransactional};
use crate::submit_query;

fn stale_mark_error(id: &str, lease: i64) -> DomainError {
    DomainError::unrecoverable(format!(
        "[LocalMessageQuery::mark] stale local message mark: id={}, lease={}",
        id, lease
    ))
}

#[instrument(err, skip(conn, form), level = Level::DEBUG)]
pub async fn append(
    conn: &mut AsyncPgConnection,
    form: &LocalMessageForm,
) -> DomainResult<LocalMessageAggr> {
    let now = OffsetDateTime::now_utc();
    let entry = LocalMessageEntry::from_form(form, now);

    diesel::insert_into(t_local_message)
        .values(&entry)
        .execute(conn)
        .await?;

    let row: LocalMessageRow = t_local_message
        .filter(f_id.eq(&entry.f_id))
        .select(LocalMessageRow::as_select())
        .first(conn)
        .await?;

    row.try_into()
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn claim(
    conn: &mut AsyncPgConnection,
    target_topic: &str,
    limit: i64,
) -> DomainResult<Vec<LocalMessageAggr>> {
    let now = OffsetDateTime::now_utc();

    // Step 1: Atomically claim message IDs with row-level locking.
    let claimed: Vec<String> = t_local_message
        .filter(f_topic.eq(target_topic))
        .filter(f_status.eq(LocalMessageStatus::Pending.as_str()))
        .filter(f_visible_at.le(now))
        .order(f_created_at.asc())
        .limit(limit)
        .select(f_id)
        .for_update()
        .skip_locked()
        .load(conn)
        .await?;

    if claimed.is_empty() {
        return Ok(Vec::new());
    }

    // Step 2: Update the claimed messages and return the updated rows.
    let no_error: Option<String> = None;
    let rows: Vec<LocalMessageRow> = diesel::update(t_local_message)
        .filter(f_id.eq_any(&claimed))
        .set((
            f_status.eq(LocalMessageStatus::Processing.as_str()),
            f_last_error.eq(no_error),
            f_lease.eq(f_lease + 1),
            f_updated_at.eq(now),
        ))
        .returning(LocalMessageRow::as_returning())
        .get_results(conn)
        .await?;

    rows.into_iter().map(TryInto::try_into).collect()
}

async fn mark_one(conn: &mut AsyncPgConnection, mark: &LocalMessageMark) -> DomainResult<()> {
    match mark {
        LocalMessageMark::Pending {
            id,
            lease,
            next_visible_at,
            last_error,
        } => {
            let row: LocalMessageRow = t_local_message
                .filter(f_id.eq(id))
                .filter(f_status.eq(LocalMessageStatus::Processing.as_str()))
                .filter(f_lease.eq(lease))
                .select(LocalMessageRow::as_select())
                .first(conn)
                .await
                .optional()?
                .ok_or_else(|| stale_mark_error(id, *lease))?;

            let now = OffsetDateTime::now_utc();

            let changes = LocalMessageAspect::new(now)
                .status(LocalMessageStatus::Pending.as_str())
                .last_error(Some(last_error))
                .retried_count(row.f_retried_count + 1)
                .visible_at(*next_visible_at);

            let affected = diesel::update(
                t_local_message
                    .filter(f_id.eq(id))
                    .filter(f_status.eq(LocalMessageStatus::Processing.as_str()))
                    .filter(f_lease.eq(lease)),
            )
            .set(&changes)
            .execute(conn)
            .await?;

            if affected == 0 {
                return Err(stale_mark_error(id, *lease));
            }
        }
        LocalMessageMark::Completed { id, lease } => {
            let now = OffsetDateTime::now_utc();
            let changes = LocalMessageAspect::new(now)
                .status(LocalMessageStatus::Completed.as_str())
                .last_error(None);

            let affected = diesel::update(
                t_local_message
                    .filter(f_id.eq(id))
                    .filter(f_status.eq(LocalMessageStatus::Processing.as_str()))
                    .filter(f_lease.eq(lease)),
            )
            .set(&changes)
            .execute(conn)
            .await?;

            if affected == 0 {
                return Err(stale_mark_error(id, *lease));
            }
        }
        LocalMessageMark::Dead {
            id,
            lease,
            last_error,
        } => {
            let now = OffsetDateTime::now_utc();
            let changes = LocalMessageAspect::new(now)
                .status(LocalMessageStatus::Dead.as_str())
                .last_error(Some(last_error));

            let affected = diesel::update(
                t_local_message
                    .filter(f_id.eq(id))
                    .filter(f_status.eq(LocalMessageStatus::Processing.as_str()))
                    .filter(f_lease.eq(lease)),
            )
            .set(&changes)
            .execute(conn)
            .await?;

            if affected == 0 {
                return Err(stale_mark_error(id, *lease));
            }
        }
    }

    Ok(())
}

#[instrument(err, skip(conn, marks), level = Level::DEBUG)]
pub async fn mark(conn: &mut AsyncPgConnection, marks: &[&LocalMessageMark]) -> DomainResult<()> {
    for mark in marks {
        mark_one(conn, mark).await?;
    }

    Ok(())
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn list_dead(
    conn: &mut AsyncPgConnection,
    target_topic: &str,
    page: Page,
) -> DomainResult<Vec<LocalMessageAggr>> {
    let rows: Vec<LocalMessageRow> = t_local_message
        .filter(f_topic.eq(target_topic))
        .filter(f_status.eq(LocalMessageStatus::Dead.as_str()))
        .order(f_updated_at.desc())
        .offset(page.offset as i64)
        .limit(page.limit as i64)
        .select(LocalMessageRow::as_select())
        .load(conn)
        .await?;

    rows.into_iter().map(TryInto::try_into).collect()
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn purge_completed(conn: &mut AsyncPgConnection, target_topic: &str) -> DomainResult<()> {
    diesel::delete(
        t_local_message
            .filter(f_topic.eq(target_topic))
            .filter(f_status.eq(LocalMessageStatus::Completed.as_str())),
    )
    .execute(conn)
    .await?;

    Ok(())
}

#[instrument(err, skip(conn, items), level = Level::DEBUG)]
pub async fn delete_dead(conn: &mut AsyncPgConnection, items: &[&str]) -> DomainResult<()> {
    for id in items {
        diesel::delete(
            t_local_message
                .filter(f_id.eq(id))
                .filter(f_status.eq(LocalMessageStatus::Dead.as_str())),
        )
        .execute(conn)
        .await?;
    }

    Ok(())
}

#[async_trait]
impl LocalMessageQuery for RdbQuery {
    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn claim(&self, topic: &str, limit: i64) -> DomainResult<Vec<LocalMessageAggr>> {
        submit_query!(self.pool, claim, topic, limit)
    }

    #[instrument(err, skip(self, marks), level = Level::DEBUG)]
    async fn mark(&self, marks: &[&LocalMessageMark]) -> DomainResult<()> {
        submit_query!(self.pool, mark, marks)
    }

    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn list_dead(&self, topic: &str, page: Page) -> DomainResult<Vec<LocalMessageAggr>> {
        submit_query!(self.pool, list_dead, topic, page)
    }

    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn purge_completed(&self, topic: &str) -> DomainResult<()> {
        submit_query!(self.pool, purge_completed, topic)
    }

    #[instrument(err, skip(self, items), level = Level::DEBUG)]
    async fn delete_dead(&self, items: &[&str]) -> DomainResult<()> {
        submit_query!(self.pool, delete_dead, items)
    }
}

#[async_trait]
impl<'c> LocalMessageQueryTransactional for RdbQueryTransactional<'c> {
    async fn append(&mut self, form: &LocalMessageForm) -> DomainResult<LocalMessageAggr> {
        append(self.conn, form).await
    }

    async fn mark_transactional(&mut self, marks: &[&LocalMessageMark]) -> DomainResult<()> {
        mark(self.conn, marks).await
    }
}
