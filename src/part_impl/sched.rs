//! Weekly database retention scheduler.

use std::time::Duration as StdDuration;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::Nucl;
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::system_mail::SystemMailEntry;
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::entity::system_mail::SystemMailRowEntry;
use crate::part_impl::repo::rdb_impl::schema::t_comic_archive::dsl::{
    f_created_at, f_team_id, t_comic_archive,
};
use crate::part_impl::repo::rdb_impl::schema::t_member::dsl::{
    f_assigned_admin_at, f_team_id as member_team_id, f_user_id, t_member,
};
use crate::part_impl::repo::rdb_impl::schema::t_system_mail;
use crate::part_impl::shared::result::diesel;
use crate::part_impl::shared::{RdbConn, RdbCore};
use crate::result::{BaseError, BaseResult, accept};
use crate::util::next_snowflake_id;

#[cfg(test)]
mod tests;

const PURGE_INTERVAL: StdDuration = StdDuration::from_secs(7 * 24 * 60 * 60);

struct ExpiredSlot {
    team_id: String,
    start: OffsetDateTime,
}

fn retained_cutoff(now: OffsetDateTime) -> BaseResult<OffsetDateTime> {
    //
    let date = Date::from_calendar_date(now.year() - 1, now.month(), 1)
        .map_err(|error| BaseError::Unrecoverable {
            message: format!(
                "[RdbSched::retained_cutoff] failed to build retention cutoff: {}",
                error
            ),
        })?;

    accept(PrimitiveDateTime::new(date, Time::MIDNIGHT).assume_utc())
}

fn next_month(start: OffsetDateTime) -> BaseResult<OffsetDateTime> {
    //
    let next = match start.month() {
        //
        Month::December => (start.year() + 1, Month::January),

        month => (
            start.year(),
            Month::try_from(u8::from(month) + 1).map_err(|error| {
                BaseError::Unrecoverable {
                    message: format!(
                        "[RdbSched::next_month] failed to build month: {}",
                        error
                    ),
                }
            })?,
        ),
    };

    let date =
        Date::from_calendar_date(next.0, next.1, 1).map_err(|error| {
            BaseError::Unrecoverable {
                message: format!(
                    "[RdbSched::next_month] failed to build month start: {}",
                    error
                ),
            }
        })?;

    accept(PrimitiveDateTime::new(date, Time::MIDNIGHT).assume_utc())
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn list_expired_slots(
    conn: &mut RdbConn,
    cutoff: OffsetDateTime,
) -> BaseResult<Vec<ExpiredSlot>> {
    //
    #[derive(QueryableByName)]
    struct ExpiredSlotRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        team_id: String,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        month_start: OffsetDateTime,
    }

    let rows: Vec<ExpiredSlotRow> = diesel::sql_query(
        "SELECT f_team_id AS team_id, date_trunc('month', f_created_at) AS month_start FROM t_comic_archive WHERE f_created_at < $1 GROUP BY f_team_id, month_start ORDER BY month_start, f_team_id",
    )
    .bind::<diesel::sql_types::Timestamptz, _>(cutoff)
    .load(conn)
    .await
    .map_err(diesel)?;

    accept(
        rows.into_iter()
            .map(|row| ExpiredSlot {
                team_id: row.team_id,
                start: row.month_start,
            })
            .collect(),
    )
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn purge_slot(
    conn: &mut RdbConn,
    slot: &ExpiredSlot,
) -> BaseResult<usize> {
    //
    let end = next_month(slot.start)?;

    let admin_ids: Vec<String> = t_member
        .filter(member_team_id.eq(&slot.team_id))
        .filter(f_assigned_admin_at.is_not_null())
        .select(f_user_id)
        .load(conn)
        .await
        .map_err(diesel)?;

    let deleted_count = diesel::delete(
        t_comic_archive
            .filter(f_team_id.eq(&slot.team_id))
            .filter(f_created_at.ge(slot.start))
            .filter(f_created_at.lt(end)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    if deleted_count == 0 {
        return accept(0);
    }

    let month = format!(
        "{:04}-{:02}",
        slot.start.year(),
        u8::from(slot.start.month())
    );

    let title = trl("mail-comic-archive-purged-title");

    let content = format!(
        "{}: team {}, month {}, comics {}",
        trl("mail-comic-archive-purged-body"),
        slot.team_id,
        month,
        deleted_count
    );

    let entries = admin_ids
        .into_iter()
        .map(|receiver_id| SystemMailEntry {
            id: next_snowflake_id(),
            receiver_id,
            title: title.clone(),
            content: content.clone(),
        })
        .collect::<Vec<_>>();

    if entries.is_empty() {
        return accept(deleted_count);
    }

    let rows = entries
        .iter()
        .map(SystemMailRowEntry::from)
        .collect::<Vec<_>>();

    diesel::insert_into(t_system_mail::table)
        .values(&rows)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(deleted_count)
}

#[instrument(level = "info", err(Debug), skip_all)]
async fn purge_once(core: &RdbCore) -> BaseResult<usize> {
    //
    let mut conn = core.get().await?;

    let cutoff = retained_cutoff(OffsetDateTime::now_utc())?;

    let slots = list_expired_slots(&mut conn, cutoff).await?;

    let mut total_deleted = 0;

    let drive = RdbDrive::new(core.clone());

    for slot in slots {
        //
        let deleted_count = drive
            .coord(async |context| purge_slot(context.conn(), &slot).await)
            .await?;

        total_deleted += deleted_count;
    }

    accept(total_deleted)
}

/// Weekly comic archive retention scheduler.
///
/// FIXME: bad naming.
pub struct RdbSched {
    /// Cancellation token to signal graceful shutdown of the scheduler.
    token: CancellationToken,
    /// Watch receiver that signals when the background loop drains.
    done: watch::Receiver<bool>,
}

impl RdbSched {
    /// Starts the retention loop and performs the first purge immediately.
    pub fn new(core: RdbCore) -> Self {
        //
        let token = CancellationToken::new();

        let (done_send, done) = watch::channel(false);

        let runner_token = token.clone();

        tokio::spawn(async move {
            //
            loop {
                //
                match purge_once(&core).await {
                    //
                    Ok(deleted_count) if deleted_count > 0 => {
                        tracing::info!(
                            deleted_count,
                            "[RdbSched::run] purged expired comic archives",
                        );
                    }

                    Ok(_) => {}

                    Err(error) => {
                        tracing::error!(
                            error = ?error,
                            "[RdbSched::run] comic archive purge failed",
                        );
                    }
                }

                tokio::select! {
                    () = runner_token.cancelled() => break,
                    () = tokio::time::sleep(PURGE_INTERVAL) => {}
                }
            }

            done_send.send_replace(true);
        });

        Self { token, done }
    }

    /// Stops the scheduler and waits for its current purge to finish.
    pub async fn close(&self) {
        //
        self.token.cancel();

        let mut done = self.done.clone();

        let _ = done.wait_for(|done| *done).await;
    }
}

impl Drop for RdbSched {
    fn drop(&mut self) {
        self.token.cancel();
    }
}
