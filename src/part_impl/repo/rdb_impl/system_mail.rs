//! RDB-backed system mail repository — free query functions and thin trait impls.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::Run;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::read::proj::system_mail::SystemMailInfo;
use crate::model::read::spec::system_mail::SystemMailListSpec;
use crate::model::write::system_mail::SystemMailEntry;
use crate::part::repo::oper::system_mail::{
    ListSystemMailInfos, MarkSystemMailRead, SendSystemMail, SendSystemMails,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::system_mail::{
    SystemMailRow, SystemMailRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_system_mail::dsl::*;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::RdbConn;
use crate::shared::result::diesel;

/// System mail RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

// ── Free functions ──────────────────────────────────────────────────────────

// Insert one system mail row and return nothing when persistence succeeds.
#[instrument(level = "info", err(Debug), skip_all)]
async fn send(conn: &mut RdbConn, entry: &SystemMailEntry) -> BaseRest<()> {
    //
    let entry = SystemMailRowEntry::from(entry);

    diesel::insert_into(t_system_mail)
        .values(&entry)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Insert one or more system mail rows with a single bulk database write.
#[instrument(level = "info", err(Debug), skip_all)]
async fn send_batch(
    conn: &mut RdbConn,
    entries: &[SystemMailEntry],
) -> BaseRest<()> {
    //
    let entries = entries
        .iter()
        .map(SystemMailRowEntry::from)
        .collect::<Vec<SystemMailRowEntry<'_>>>();

    diesel::insert_into(t_system_mail)
        .values(&entries)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Query mails for the receiver, applying read-state filters and pagination.
#[instrument(level = "info", err(Debug), skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &SystemMailListSpec,
) -> BaseRest<Vec<SystemMailInfo>> {
    //
    let mut query = t_system_mail
        .filter(f_receiver_id.eq(spec.receiver_id.as_str()))
        .select(SystemMailRow::as_select())
        .into_boxed();

    query = match spec.is_read {
        //
        Some(is_read) => query.filter(f_read.eq(is_read)),

        None => query,
    };

    let rows: Vec<SystemMailRow> = query
        .order_by(f_created_at.desc())
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

// Validate ownership, then flip `read` for a mail belonging to the receiver.
#[instrument(level = "info", err(Debug), skip_all)]
async fn mark_read(
    conn: &mut RdbConn,
    id: &str,
    user_id: &str,
) -> BaseRest<()> {
    //
    let row: Option<SystemMailRow> = t_system_mail
        .filter(f_id.eq(id))
        .select(SystemMailRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let mail = match row {
        //
        Some(mail) => mail,

        None => {
            //
            let message = trl("error-system-mail-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %message,
                system_mail_id = %id,
                receiver_user_id = %user_id,
                operation = "mark system mail read",
                "expected system mail error",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message,
            });
        }
    };

    if mail.f_receiver_id != user_id {
        //
        let message = "error-forbidden".to_owned();

        tracing::warn!(
            error_variant = ?ExpectedVariant::Perm,
            err_message = %message,
            system_mail_id = %id,
            receiver_user_id = %user_id,
            actual_receiver_user_id = %mail.f_receiver_id,
            operation = "mark system mail read",
            "expected system mail permission error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message,
        });
    }

    diesel::update(t_system_mail.filter(f_id.eq(id)))
        .set(f_read.eq(true))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

impl Run<SendSystemMail<'_>> for RdbRepo {
    // Reuse base error type for send operations.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Persist one outbound system mail entry in the request-scoped transaction.
    async fn run(&self, oper: &SendSystemMail<'_>) -> BaseRest<()> {
        submit_query!(self.core, send, oper.entry)
    }
}

impl Run<SendSystemMails<'_>> for RdbRepo {
    // Reuse base error type for bulk send operations.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Persist multiple outbound system mails in one request scope.
    async fn run(&self, oper: &SendSystemMails<'_>) -> BaseRest<()> {
        submit_query!(self.core, send_batch, oper.entries)
    }
}

impl Run<ListSystemMailInfos<'_>> for RdbRepo {
    // Reuse base error type for listing operations.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Query and return a paginated view of user-targeted system mails.
    async fn run(
        &self,
        oper: &ListSystemMailInfos<'_>,
    ) -> BaseRest<Vec<SystemMailInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl Run<MarkSystemMailRead<'_>> for RdbRepo {
    // Reuse base error type for read-mark operations.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Verify receiver ownership then set the target mail as read.
    async fn run(&self, oper: &MarkSystemMailRead<'_>) -> BaseRest<()> {
        submit_query!(self.core, mark_read, oper.id, oper.user_id)
    }
}
