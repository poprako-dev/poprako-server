//! RDB-backed system mail repository — free query functions and thin trait impls.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::model::system_mail_model;
use crate::part::repo::step::system_mail::{
    ListInfosByReceiverId, MarkRead, Send, SendBatch,
};
use crate::part::repo::system_mail::{
    SystemMailRepo, SystemMailRepoTransactional,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::rdb_impl::entity::system_mail::{
    SystemMailEntry, SystemMailRow,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, RdbRepoTransactional};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{ExpectedVariant, RegularError, RegularResult};

use crate::part_impl::repo::rdb_impl::schema::t_system_mail::dsl::*;

impl SystemMailRepo<RdbContext> for RdbRepo {}

impl SystemMailRepoTransactional<RdbContext> for RdbRepoTransactional {}

// ── Free functions ──────────────────────────────────────────────────────────

/// Send a single system mail by inserting its row.
async fn send(
    conn: &mut RdbConn,
    form: &system_mail_model::Form,
) -> RegularResult<()> {
    //
    let entry = SystemMailEntry::from(form);

    diesel::insert_into(t_system_mail)
        .values(&entry)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Batch-send system mail by inserting rows for every form.
async fn send_batch(
    conn: &mut RdbConn,
    forms: &[system_mail_model::Form],
) -> RegularResult<()> {
    //
    let entries: Vec<SystemMailEntry<'_>> =
        forms.iter().map(SystemMailEntry::from).collect();

    diesel::insert_into(t_system_mail)
        .values(&entries)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Query system mail for a receiver, optionally filtered by read status.
async fn list_infos(
    conn: &mut RdbConn,
    receiver_id: &str,
    read: Option<bool>,
    offset: u32,
    limit: u32,
) -> RegularResult<Vec<system_mail_model::Info>> {
    //
    let mut query = t_system_mail
        .filter(f_receiver_id.eq(receiver_id))
        .select(SystemMailRow::as_select())
        .into_boxed();

    if let Some(read) = read {
        query = query.filter(f_read.eq(read));
    }

    let rows: Vec<SystemMailRow> = query
        .order_by(f_created_at.desc())
        .offset(offset as i64)
        .limit(limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Mark a system mail as read, authorizing by the owning receiver.
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

// ── Non-transactional: Execute impls ─────────────────────────────────────────

#[async_trait]
impl<'a> Execute<Send<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &Send<'a>) -> RegularResult<()> {
        submit_query!(self.core, send, step.form)
    }
}

#[async_trait]
impl<'a> Execute<SendBatch<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &SendBatch<'a>) -> RegularResult<()> {
        submit_query!(self.core, send_batch, step.forms)
    }
}

#[async_trait]
impl<'a> Execute<ListInfosByReceiverId<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfosByReceiverId<'a>,
    ) -> RegularResult<Vec<system_mail_model::Info>> {
        submit_query!(
            self.core,
            list_infos,
            step.receiver_id,
            step.read,
            step.offset,
            step.limit
        )
    }
}

#[async_trait]
impl<'a> Execute<MarkRead<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &MarkRead<'a>) -> RegularResult<()> {
        submit_query!(self.core, mark_read, step.id, step.user_id)
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;
