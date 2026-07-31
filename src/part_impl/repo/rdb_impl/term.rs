//! Diesel-backed terminology-entry repository operations.

use diesel::{
    ExpressionMethods as _, OptionalExtension as _,
    PgTextExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::read::proj::term::TermInfo;
use crate::model::read::spec::term::TermListSpec;
use crate::model::write::term::{TermEntry, TermRepl};
use crate::part::repo::oper::term::{
    CreateTerm, DeleteTerm, DeleteTerms, GetTermInfo, GetTermInfoExcluded,
    ListTermInfos, LockTerm, UpdateTerm,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::term::{TermRow, TermRowEntry};
use crate::part_impl::repo::rdb_impl::schema::t_term::dsl::*;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;
use crate::shared::{RdbConn, RdbContext};

/// Term RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

impl Run<GetTermInfo<'_>> for RdbRepo {
    // Map `GetTermInfo` to repository orchestration without ambient transaction.
    type Error = BaseError;

    // Resolve one term info by id through the submit-query entrypoint.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &GetTermInfo<'_>) -> BaseRest<TermInfo> {
        submit_query!(self.core, get_info, oper.id)
    }
}

impl Run<ListTermInfos<'_>> for RdbRepo {
    // Map `ListTermInfos` to repository orchestration without ambient transaction.
    type Error = BaseError;

    // Resolve a pageable list from the supplied term list spec.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &ListTermInfos<'_>) -> BaseRest<Vec<TermInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl Step<CreateTerm<'_>, RdbContext> for RdbRepo {
    // Create a term row inside an active transaction boundary.
    type Error = BaseError;

    // Convert the request payload and insert it with immediate return of inserted info.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateTerm<'_>,
    ) -> BaseRest<TermInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl Step<GetTermInfoExcluded<'_>, RdbContext> for RdbRepo {
    // Read a term for exclusive use inside an active transaction context.
    type Error = BaseError;

    // Resolve one term with `FOR UPDATE` semantics for downstream mutation.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetTermInfoExcluded<'_>,
    ) -> BaseRest<TermInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl Step<LockTerm<'_>, RdbContext> for RdbRepo {
    // Acquire a row-level lock for a term within a transaction.
    type Error = BaseError;

    // Lock the target term so the next mutation in the transaction is serialized.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &LockTerm<'_>,
    ) -> BaseRest<()> {
        lock_term(context.conn(), oper.id).await
    }
}

impl Step<UpdateTerm<'_>, RdbContext> for RdbRepo {
    // Apply term info updates inside an active transaction boundary.
    type Error = BaseError;

    // Forward update payload into DB update clause and keep updated-at fresh.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateTerm<'_>,
    ) -> BaseRest<()> {
        update_info(context.conn(), oper.update).await
    }
}

impl Step<DeleteTerm<'_>, RdbContext> for RdbRepo {
    // Remove one term row inside an active transaction boundary.
    type Error = BaseError;

    // Execute hard delete for the target term id.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteTerm<'_>,
    ) -> BaseRest<()> {
        delete(context.conn(), oper.id).await
    }
}

impl Step<DeleteTerms<'_>, RdbContext> for RdbRepo {
    // Remove all terms for a termbase inside an active transaction boundary.
    type Error = BaseError;

    // Cascade-like cleanup behavior is implemented as bulk delete filtered by termbase.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteTerms<'_>,
    ) -> BaseRest<()> {
        delete_terms(context.conn(), oper.termbase_id).await
    }
}

// Delete one term by id.
#[instrument(level = "info", err(Debug), skip_all)]
async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    // Remove the single term row and return once persistence succeeds.
    diesel::delete(t_term.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Escape wildcard characters before reusing user input in a SQL `ILIKE` pattern.
fn escape_ilike_pattern(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

// Delete all terms that belong to one termbase.
#[instrument(level = "info", err(Debug), skip_all)]
async fn delete_terms(conn: &mut RdbConn, termbase_id: &str) -> BaseRest<()> {
    //
    // Remove dependency rows in bulk when the parent termbase is being cleaned up.
    diesel::delete(t_term.filter(f_termbase_id.eq(termbase_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Insert a new term row and return the inserted row as response info.
#[instrument(level = "info", err(Debug), skip_all)]
async fn create(
    conn: &mut RdbConn,
    term_entry: &TermEntry,
) -> BaseRest<TermInfo> {
    //
    // Convert API entry to DB row shape and rely on returning() to fetch the saved state.
    let entry = TermRowEntry::from(term_entry);

    let row: TermRow = diesel::insert_into(t_term)
        .values(&entry)
        .returning(TermRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    accept(row.into())
}

// Load one term row by id in a lock-compatible path and convert it to response info.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_excluded(conn: &mut RdbConn, id: &str) -> BaseRest<TermInfo> {
    //
    // Use `for_update()` to prevent concurrent updates while resolving this term.
    let row: Option<TermRow> = t_term
        .filter(f_id.eq(id))
        .select(TermRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let row = match row {
        //
        Some(row) => row,

        None => {
            //
            let message = trl("error-term-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %message,
                term_id = %id,
                operation = "get locked term info",
                "expected term error",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message,
            });
        }
    };

    accept(row.into())
}

// Locks a term row for mutation safety.
#[instrument(level = "info", err(Debug), skip_all)]
async fn lock_term(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    // Confirm existence and keep the row locked until the current transaction ends.
    let row: Option<String> = t_term
        .filter(f_id.eq(id))
        .select(f_id)
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let _ = match row {
        //
        Some(row) => row,

        None => {
            //
            let message = trl("error-term-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %message,
                term_id = %id,
                operation = "lock term row",
                "expected term error",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message,
            });
        }
    };

    accept(())
}

// Apply a partial update payload to an existing term row.
#[instrument(level = "info", err(Debug), skip_all)]
async fn update_info(conn: &mut RdbConn, update: &TermRepl) -> BaseRest<()> {
    //
    // Prepare nullable target entries and write all requested fields in one update.
    let targets = update
        .targets
        .iter()
        .map(|target| Some(target.as_str()))
        .collect::<Vec<_>>();

    diesel::update(t_term.filter(f_id.eq(&update.id)))
        .set((
            f_source.eq(&update.source),
            f_targets.eq(targets),
            f_comment.eq(update.comment.as_deref()),
            f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Build a filtered query for term listing and execute a paged query.
#[instrument(level = "info", err(Debug), skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &TermListSpec,
) -> BaseRest<Vec<TermInfo>> {
    //
    // Start with a termbase constraint, then apply optional fuzzy source matching.
    let mut query = t_term
        .filter(f_termbase_id.eq(&spec.termbase_id))
        .select(TermRow::as_select())
        .into_boxed();

    if let Some(fuzzy_source) = &spec.fuzzy_source {
        //
        let escaped = escape_ilike_pattern(fuzzy_source);

        let pattern = format!("%{}%", escaped);

        query = query.filter(f_source.ilike(pattern));
    }

    let rows: Vec<TermRow> = query
        .order_by(f_updated_at.desc())
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

// Load one term row by id and convert it into response info.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info(conn: &mut RdbConn, id: &str) -> BaseRest<TermInfo> {
    //
    // Read the row with strict id match and convert it to a rich term view.
    let row: Option<TermRow> = t_term
        .filter(f_id.eq(id))
        .select(TermRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let row = match row {
        //
        Some(row) => row,

        None => {
            //
            let message = trl("error-term-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %message,
                term_id = %id,
                operation = "get term info",
                "expected term error",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message,
            });
        }
    };

    accept(row.into())
}
