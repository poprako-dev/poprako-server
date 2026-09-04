//! RDB-backed system mail repository — free query functions and thin trait impls.

/// System mail RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use std::collections::HashMap;

use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::Run;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::system_mail::SystemMailInfo;
use crate::model::read::spec::system_mail::SystemMailListSpec;
use crate::model::write::system_mail::SystemMailEntry;
use crate::part::repo::oper::system_mail::{
    ListSystemMailInfos, MarkSystemMailsRead, SendSystemMail, SendSystemMails,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::system_mail::{
    SystemMailEntryRow, SystemMailInfoRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_system_mail::dsl::{
    f_created_at, f_id, f_read, f_receiver_id, t_system_mail,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;

// ── Free functions ──────────────────────────────────────────────────────────

// Insert one system mail row and return nothing when persistence succeeds.
#[instrument(level = "info", skip_all)]
async fn send(conn: &mut RdbConn, entry: &SystemMailEntry) -> BaseRest<()> {
    //
    let entry = SystemMailEntryRow::from(entry);

    diesel::insert_into(t_system_mail)
        .values(&entry)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Insert one or more system mail rows with a single bulk database write.
#[instrument(level = "info", skip_all)]
async fn send_batch(
    conn: &mut RdbConn,
    entries: &[SystemMailEntry],
) -> BaseRest<()> {
    //
    let entries = entries
        .iter()
        .map(SystemMailEntryRow::from)
        .collect::<Vec<SystemMailEntryRow<'_>>>();

    diesel::insert_into(t_system_mail)
        .values(&entries)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Query mails for the receiver, applying read-state filters and pagination.
#[instrument(level = "info", skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &SystemMailListSpec,
) -> BaseRest<Vec<SystemMailInfo>> {
    //
    let mut query = t_system_mail
        .filter(f_receiver_id.eq(spec.receiver_id.as_str()))
        .select(SystemMailInfoRow::as_select())
        .into_boxed();

    query = match spec.is_read {
        //
        Some(is_read) => query.filter(f_read.eq(is_read)),

        None => query,
    };

    let rows = query
        .order_by(f_created_at.desc())
        .offset(i64::from(spec.offset))
        .limit(i64::from(spec.limit.get()))
        .load::<SystemMailInfoRow>(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

// Validate ownership for the complete batch, then mark it read with one update.
#[instrument(level = "info", skip_all)]
async fn mark_read_batch(
    conn: &mut RdbConn,
    ids: &[String],
    user_id: &str,
) -> BaseRest<()> {
    //
    let receiver_ids = t_system_mail
        .filter(f_id.eq_any(ids))
        .select((f_id, f_receiver_id))
        .load::<(String, String)>(conn)
        .await
        .map_err(diesel)?;

    let receiver_ids_by_system_mail_id =
        receiver_ids.into_iter().collect::<HashMap<_, _>>();

    for id in ids {
        //
        let Some(receiver_id) = receiver_ids_by_system_mail_id.get(id) else {
            //
            let message = trl("error-system-mail-not-found");

            tracing::warn!(
                err_variant = ?ExpectedVariant::Args,
                err_message = %message,
                system_mail_id = %id,
                receiver_user_id = %user_id,
                operation = "mark system mails read",
                "expected system mail error",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message,
            });
        };

        if receiver_id != user_id {
            //
            let message = "error-forbidden".to_owned();

            tracing::warn!(
                err_variant = ?ExpectedVariant::Perm,
                err_message = %message,
                system_mail_id = %id,
                receiver_user_id = %user_id,
                actual_receiver_user_id = %receiver_id,
                operation = "mark system mails read",
                "expected system mail perm error",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                message,
            });
        }
    }

    diesel::update(
        t_system_mail
            .filter(f_id.eq_any(ids))
            .filter(f_receiver_id.eq(user_id)),
    )
    .set(f_read.eq(true))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

impl Run<SendSystemMail<'_>> for HybRepo {
    // Reuse base error type for send operations.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Persist one outbound system mail entry in the request-scoped transaction.
    async fn run(&self, oper: &SendSystemMail<'_>) -> BaseRest<()> {
        submit_query!(self.rdb_core, send, oper.entry)
    }
}

impl Run<SendSystemMails<'_>> for HybRepo {
    // Reuse base error type for bulk send operations.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Persist multiple outbound system mails in one request scope.
    async fn run(&self, oper: &SendSystemMails<'_>) -> BaseRest<()> {
        submit_query!(self.rdb_core, send_batch, oper.entries)
    }
}

impl Run<ListSystemMailInfos<'_>> for HybRepo {
    // Reuse base error type for listing operations.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Query and return a paginated view of user-targeted system mails.
    async fn run(
        &self,
        oper: &ListSystemMailInfos<'_>,
    ) -> BaseRest<Vec<SystemMailInfo>> {
        submit_query!(self.rdb_core, list_infos, oper.spec)
    }
}

impl Run<MarkSystemMailsRead<'_>> for HybRepo {
    // Reuse base error type for read-mark operations.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Verify receiver ownership then set the target mail as read.
    async fn run(&self, oper: &MarkSystemMailsRead<'_>) -> BaseRest<()> {
        submit_query!(self.rdb_core, mark_read_batch, oper.ids, oper.user_id)
    }
}
