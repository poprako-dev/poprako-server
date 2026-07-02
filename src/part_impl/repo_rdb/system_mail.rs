//! RDB-backed system mail repository — free query functions and thin trait impls.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::model::system_mail::{SystemMailForm, SystemMailInfo};
use crate::part::repo::step::system_mail::{ListInfosByReceiverId, MarkRead, Send, SendBatch};
use crate::part::repo::system_mail::{SystemMailRepo, SystemMailRepoTransactional};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_rdb::dsl;
use crate::part_impl::repo_rdb::entity::system_mail::{SystemMailEntry, SystemMailRow};
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional};
use crate::part_impl::shared_rdb::RdbConn;
use crate::part_impl::shared_rdb::RdbContext;
use crate::part_impl::shared_rdb::result::{diesel, expected};
use crate::result::{ExpectedVariant, RegularError, RegularResult};

// NOTE: use dsl::* is the Diesel impl layer exception to rust-use-style
use dsl::t_system_mail::*;

impl SystemMailRepo<RdbContext> for RdbRepo {}

impl SystemMailRepoTransactional<RdbContext> for RdbRepoTransactional {}

// ── Free functions ──────────────────────────────────────────────────────────

async fn send(conn: &mut RdbConn, form: &SystemMailForm) -> RegularResult<()> {
    let entry = SystemMailEntry::from(form);

    diesel::insert_into(t_system_mail)
        .values(&entry)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn send_batch(conn: &mut RdbConn, forms: &[SystemMailForm]) -> RegularResult<()> {
    let entries: Vec<SystemMailEntry<'_>> = forms.iter().map(SystemMailEntry::from).collect();

    diesel::insert_into(t_system_mail)
        .values(&entries)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

// FIXME: list should have ONLY ONE ENTRY!!!!

async fn list_infos(
    conn: &mut RdbConn,
    receiver_id: &str,
    read: Option<bool>,
    offset: u64,
    limit: u64,
) -> RegularResult<Vec<SystemMailInfo>> {
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

async fn mark_read(conn: &mut RdbConn, id: &str, user_id: &str) -> RegularResult<()> {
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
            variant: ExpectedVariant::PermDeny,
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
        submit_query!(self.shared, send, step.form)
    }
}

#[async_trait]
impl<'a> Execute<SendBatch<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &SendBatch<'a>) -> RegularResult<()> {
        submit_query!(self.shared, send_batch, step.forms)
    }
}

#[async_trait]
impl<'a> Execute<ListInfosByReceiverId<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfosByReceiverId<'a>,
    ) -> RegularResult<Vec<SystemMailInfo>> {
        submit_query!(
            self.shared,
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
        submit_query!(self.shared, mark_read, step.id, step.user_id)
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;
