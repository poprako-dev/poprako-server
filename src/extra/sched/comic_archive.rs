//! Periodic retention job for expired comic archives.

use std::collections::BTreeSet;
use std::time::Duration;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::Nucl;
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::write::system_mail::SystemMailEntry;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::rdb_impl::entity::system_mail::SystemMailEntryRow;
use crate::part_impl::repo::rdb_impl::schema::t_comic_archive::dsl::{
    f_created_at, f_team_id, t_comic_archive,
};
use crate::part_impl::repo::rdb_impl::schema::t_member::dsl::{
    f_assigned_admin_at, f_team_id as member_team_id, f_user_id, t_member,
};
use crate::part_impl::repo::rdb_impl::schema::t_system_mail;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::result::diesel;
use crate::shared::{RdbConn, RdbCore};
use crate::util::next_snowflake_id;

#[cfg(test)]
// Test module for retention schedule coverage scenarios.
mod tests;

/// Spawns the comic archiver background task.
pub fn spawn(core: RdbCore, token: CancellationToken) -> watch::Receiver<bool> {
    //
    let (done_send, done_recv) = watch::channel(false);

    tokio::spawn(async move {
        //
        loop {
            //
            match purge_once(&core).await {
                //
                Ok(deleted_count) if deleted_count > 0 => {
                    tracing::info!(
                        deleted_count,
                        "[ComicArchiveRetention::run] purged expired archives",
                    );
                }

                Ok(_) => {}

                Err(error) => {
                    tracing::error!(
                        err = ?error,
                        "[ComicArchiveRetention::run] retention job failed",
                    );
                }
            }

            tokio::select! {
                () = token.cancelled() => break,
                () = tokio::time::sleep(PURGE_INTERVAL) => {}
            }
        }

        done_send.send_replace(true);
    });

    done_recv
}

// Retention retention cycle frequency.
const PURGE_INTERVAL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

// Cached team/month slot to purge archive rows safely in batches.
struct ExpiredSlot {
    //
    // Team being iterated for archive retention.
    team_id: String,

    // Month start time that defines the purge window.
    start: OffsetDateTime,
}

#[instrument(level = "info", skip_all)]
// Purge all retained-expired slots for one sweep and report total deletions.
async fn purge_once(core: &RdbCore) -> BaseRest<usize> {
    //
    let mut conn = core.get().await?;

    let cutoff = retained_cutoff(OffsetDateTime::now_utc())?;

    let slots = list_expired_slots(&mut conn, cutoff).await?;

    let mut total_deleted = 0;

    let nucl = RdbNucl::new(core.clone());

    for slot in slots {
        //
        let deleted_count = nucl
            .coord(async |context| purge_slot(context.conn(), &slot).await)
            .await?;

        total_deleted += deleted_count;
    }

    accept(total_deleted)
}

// Keep archive entries older than last year from the first day of cutoff month.
fn retained_cutoff(now: OffsetDateTime) -> BaseRest<OffsetDateTime> {
    //
    let date = Date::from_calendar_date(now.year() - 1, now.month(), 1)
        .map_err(|error| BaseError::Unrecoverable {
            message: format!(
                "[ComicArchiveRetention::retained_cutoff] failed to build cutoff: {}",
                error
            ),
        })?;

    accept(PrimitiveDateTime::new(date, Time::MIDNIGHT).assume_utc())
}

#[instrument(level = "info", skip_all)]
// Query months already expired for retention and deduplicate by team/month.
async fn list_expired_slots(
    conn: &mut RdbConn,
    cutoff: OffsetDateTime,
) -> BaseRest<Vec<ExpiredSlot>> {
    //
    let rows = t_comic_archive
        .filter(f_created_at.lt(cutoff))
        .select((f_team_id, f_created_at))
        .order_by((f_created_at.asc(), f_team_id.asc()))
        .load::<(String, OffsetDateTime)>(conn)
        .await
        .map_err(diesel)?;

    let mut unique_slots = BTreeSet::new();

    for (team_id, created_at) in rows {
        unique_slots.insert((month_start(created_at)?, team_id));
    }

    accept(
        unique_slots
            .into_iter()
            .map(|(start, team_id)| ExpiredSlot { team_id, start })
            .collect(),
    )
}

#[instrument(level = "info", skip_all)]
// Delete archived rows for one team-month slot and notify team admins.
async fn purge_slot(conn: &mut RdbConn, slot: &ExpiredSlot) -> BaseRest<usize> {
    //
    let end = next_month(slot.start)?;

    let admin_ids = t_member
        .filter(member_team_id.eq(&slot.team_id))
        .filter(f_assigned_admin_at.is_not_null())
        .select(f_user_id)
        .load::<String>(conn)
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

    let (title, content) = (
        trl("mail-comic-archive-purged-title"),
        format!(
            "{}: team {}, month {}, comics {}",
            trl("mail-comic-archive-purged-body"),
            slot.team_id,
            month,
            deleted_count
        ),
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
        .map(SystemMailEntryRow::from)
        .collect::<Vec<_>>();

    diesel::insert_into(t_system_mail::table)
        .values(&rows)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(deleted_count)
}

// Compute midnight UTC at the first day of the current month.
fn month_start(timestamp: OffsetDateTime) -> BaseRest<OffsetDateTime> {
    //
    let date = Date::from_calendar_date(
        timestamp.year(),
        timestamp.month(),
        1,
    )
    .map_err(|error| BaseError::Unrecoverable {
        message: format!(
            "[ComicArchiveRetention::month_start] failed to build month: {}",
            error
        ),
    })?;

    accept(PrimitiveDateTime::new(date, Time::MIDNIGHT).assume_utc())
}

// Compute the first-day midnight of the following month.
fn next_month(start: OffsetDateTime) -> BaseRest<OffsetDateTime> {
    //
    let next = match start.month() {
        //
        Month::December => (start.year() + 1, Month::January),

        month => (
            start.year(),
            Month::try_from(u8::from(month) + 1).map_err(|error| {
                BaseError::Unrecoverable {
                    message: format!(
                        "[ComicArchiveRetention::next_month] failed to build month: {}",
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
                    "[ComicArchiveRetention::next_month] failed to build month start: {}",
                    error
                ),
            }
        })?;

    accept(PrimitiveDateTime::new(date, Time::MIDNIGHT).assume_utc())
}
