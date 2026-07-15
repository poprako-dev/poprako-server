//! RDB-backed system mail repository — free query functions and thin trait impls.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::Run;

use tracing::instrument;

use crate::model::system_mail::{
    SystemMailEntry, SystemMailInfo, SystemMailInfoListKind,
    SystemMailInfoListSpec,
};
use crate::part::repo::oper::system_mail::{
    ListSystemMailInfos, MarkSystemMailRead, SendSystemMail, SendSystemMails,
};
use crate::part::repo::system_mail::SystemMailRepo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::system_mail::{
    SystemMailRow, SystemMailRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_system_mail::dsl::*;
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{ExpectedVariant, RegularError, RegularResult};

impl SystemMailRepo<RdbContext> for RdbRepo {}

// ── Free functions ──────────────────────────────────────────────────────────

/// Send a single system mail by inserting its row.
#[instrument(level = "info", err(Debug), skip_all)]
async fn send(
    conn: &mut RdbConn,
    entry: &SystemMailEntry,
) -> RegularResult<()> {
    //
    let entry = SystemMailRowEntry::from(entry);

    diesel::insert_into(t_system_mail)
        .values(&entry)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Batch-send system mail by inserting rows for every entry.
#[instrument(level = "info", err(Debug), skip_all)]
async fn send_batch(
    conn: &mut RdbConn,
    entries: &[SystemMailEntry],
) -> RegularResult<()> {
    //
    let entries: Vec<SystemMailRowEntry<'_>> =
        entries.iter().map(SystemMailRowEntry::from).collect();

    diesel::insert_into(t_system_mail)
        .values(&entries)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Query system mail selected by a list specification.
#[instrument(level = "info", err(Debug), skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &SystemMailInfoListSpec,
) -> RegularResult<Vec<SystemMailInfo>> {
    //
    let mut query = t_system_mail
        .filter(f_receiver_id.eq(spec.receiver_id.as_str()))
        .select(SystemMailRow::as_select())
        .into_boxed();

    query = match &spec.kind {
        //
        SystemMailInfoListKind::All => query,

        SystemMailInfoListKind::Read => query.filter(f_read.eq(true)),

        SystemMailInfoListKind::Unread => query.filter(f_read.eq(false)),
    };

    let rows: Vec<SystemMailRow> = query
        .order_by(f_created_at.desc())
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Mark a system mail as read, authorizing by the owning receiver.
#[instrument(level = "info", err(Debug), skip_all)]
async fn mark_read(
    conn: &mut RdbConn,
    id: &str,
    user_id: &str,
) -> RegularResult<()> {
    //
    let row: Option<SystemMailRow> = t_system_mail
        .filter(f_id.eq(id))
        .select(SystemMailRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let mail = row.ok_or_else(|| expected("error-system-mail-not-found"))?;

    if mail.f_receiver_id != user_id {
        return Err(RegularError::Expected {
            variant: ExpectedVariant::Perm,
            message: "error-forbidden".into(),
        });
    }

    diesel::update(t_system_mail.filter(f_id.eq(id)))
        .set(f_read.eq(true))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

impl Run<SendSystemMail<'_>> for RdbRepo {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &SendSystemMail<'_>) -> RegularResult<()> {
        submit_query!(self.core, send, oper.entry)
    }
}

impl Run<SendSystemMails<'_>> for RdbRepo {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &SendSystemMails<'_>) -> RegularResult<()> {
        submit_query!(self.core, send_batch, oper.entries)
    }
}

impl Run<ListSystemMailInfos<'_>> for RdbRepo {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListSystemMailInfos<'_>,
    ) -> RegularResult<Vec<SystemMailInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl Run<MarkSystemMailRead<'_>> for RdbRepo {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &MarkSystemMailRead<'_>) -> RegularResult<()> {
        submit_query!(self.core, mark_read, oper.id, oper.user_id)
    }
}

#[cfg(all(test, feature = "repo"))]
mod tests;
