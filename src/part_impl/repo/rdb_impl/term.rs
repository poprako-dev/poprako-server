//! Diesel-backed terminology-entry repository operations.

/// Term RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use diesel::{
    ExpressionMethods as _, OptionalExtension as _,
    PgTextExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::read::proj::term::TermInfo;
use crate::model::write::term::{TermEntry, TermRepl};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::term::{
    CreateTerm, DeleteTerm, DeleteTerms, GetTermInfo, GetTermInfoExcluded,
    ListTermInfos, LockTerm, UpdateTerm, UpsertTerms,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::term::{
    TermEntryRow, TermInfoRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_term::dsl::{
    f_comment, f_id, f_source, f_targets, f_termbase_id, f_updated_at, t_term,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;
use crate::shared::{RdbConn, RdbContext};

impl Run<GetTermInfo<'_>> for HybRepo {
    // Map `GetTermInfo` to repository orchestration without ambient transaction.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve one term info by id through the submit-query entrypoint.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &GetTermInfo<'_>) -> BaseRest<TermInfo> {
        submit_query!(self.core, get_info, oper.id)
    }
}

impl Run<ListTermInfos<'_>> for HybRepo {
    // Map `ListTermInfos` to repository orchestration without ambient transaction.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Route list variants to their corresponding query shape.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &ListTermInfos<'_>) -> BaseRest<Vec<TermInfo>> {
        //
        match oper {
            //
            ListTermInfos::Query {
                termbase_id,
                fuzzy_source,
                offset,
                limit,
            } => {
                //
                submit_query!(
                    self.core,
                    list_infos,
                    termbase_id,
                    *fuzzy_source,
                    *offset,
                    *limit
                )
            }

            ListTermInfos::Termbase { termbase_id } => {
                submit_query!(self.core, list_all_infos, termbase_id)
            }
        }
    }
}

impl<L> Step<CreateTerm<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Create a term row inside an active transaction boundary.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Convert the request payload and insert it with immediate return of inserted info.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateTerm<'_>,
    ) -> BaseRest<TermInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<L> Step<ListTermInfos<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Read selected terms inside the caller's transaction snapshot.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Route list variants to their corresponding query shape.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListTermInfos<'_>,
    ) -> BaseRest<Vec<TermInfo>> {
        //
        match oper {
            //
            ListTermInfos::Query {
                termbase_id,
                fuzzy_source,
                offset,
                limit,
            } => {
                //
                list_infos(
                    context.conn(),
                    termbase_id,
                    *fuzzy_source,
                    *offset,
                    *limit,
                )
                .await
            }

            ListTermInfos::Termbase { termbase_id } => {
                list_all_infos(context.conn(), termbase_id).await
            }
        }
    }
}

impl<L> Step<GetTermInfoExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Read a term for exclusive use inside an active transaction context.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve one term with `FOR UPDATE` semantics for downstream mutation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetTermInfoExcluded<'_>,
    ) -> BaseRest<TermInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl<L> Step<LockTerm<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Acquire a row-level lock for a term within a transaction.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Lock the target term so the next mutation in the transaction is serialized.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &LockTerm<'_>,
    ) -> BaseRest<()> {
        lock_term(context.conn(), oper.id).await
    }
}

impl<L> Step<UpdateTerm<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Apply term info updates inside an active transaction boundary.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Forward update payload into DB update clause and keep updated-at fresh.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdateTerm<'_>,
    ) -> BaseRest<()> {
        update_info(context.conn(), oper.update).await
    }
}

impl<L> Step<UpsertTerms<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Apply one bounded terminology import inside the caller's transaction.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Apply imported entries and updates inside the active transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpsertTerms<'_>,
    ) -> BaseRest<()> {
        upsert_terms(context.conn(), oper.entries, oper.updates).await
    }
}

impl<L> Step<DeleteTerm<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Remove one term row inside an active transaction boundary.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Execute hard delete for the target term id.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeleteTerm<'_>,
    ) -> BaseRest<()> {
        delete(context.conn(), oper.id).await
    }
}

impl<L> Step<DeleteTerms<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Remove all terms for a termbase inside an active transaction boundary.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Cascade-like cleanup behavior is implemented as bulk delete filtered by termbase.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeleteTerms<'_>,
    ) -> BaseRest<()> {
        delete_terms(context.conn(), oper.termbase_id).await
    }
}

// Delete one term by id.
#[instrument(level = "info", skip_all)]
async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    // Remove the single term row and return once persistence succeeds.
    diesel::delete(t_term.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Apply a partial update payload to an existing term row.
#[instrument(level = "info", skip_all)]
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

// Escape wildcard characters before reusing user input in a SQL `ILIKE` pattern.
fn escape_ilike_pattern(input: &str) -> String {
    //
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

// Delete all terms that belong to one termbase.
#[instrument(level = "info", skip_all)]
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
#[instrument(level = "info", skip_all)]
async fn create(
    conn: &mut RdbConn,
    term_entry: &TermEntry,
) -> BaseRest<TermInfo> {
    //
    // Convert API entry to DB row shape and rely on returning() to fetch the saved state.
    let entry = TermEntryRow::from(term_entry);

    let row = diesel::insert_into(t_term)
        .values(&entry)
        .returning(TermInfoRow::as_returning())
        .get_result::<TermInfoRow>(conn)
        .await
        .map_err(diesel)?;

    accept(row.into())
}

// Load every terminology entry in deterministic source order.
#[instrument(level = "info", skip_all)]
async fn list_all_infos(
    conn: &mut RdbConn,
    termbase_id: &str,
) -> BaseRest<Vec<TermInfo>> {
    //
    let rows = t_term
        .filter(f_termbase_id.eq(termbase_id))
        .order_by((f_source.asc(), f_id.asc()))
        .select(TermInfoRow::as_select())
        .load::<TermInfoRow>(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

// Apply bounded imported inserts and replacements inside one adapter step.
#[instrument(level = "info", skip_all)]
async fn upsert_terms(
    conn: &mut RdbConn,
    entries: &[TermEntry],
    updates: &[TermRepl],
) -> BaseRest<()> {
    //
    if !entries.is_empty() {
        //
        let rows = entries.iter().map(TermEntryRow::from).collect::<Vec<_>>();

        diesel::insert_into(t_term)
            .values(&rows)
            .execute(conn)
            .await
            .map_err(diesel)?;
    }

    for update in updates {
        update_info(conn, update).await?;
    }

    accept(())
}

// Locks a term row for mutation safety.
#[instrument(level = "info", skip_all)]
async fn lock_term(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    // Confirm existence and keep the row locked until the current transaction ends.
    let row = t_term
        .filter(f_id.eq(id))
        .select(f_id)
        .for_update()
        .get_result::<String>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(_) = row else {
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
    };

    accept(())
}

// Load one term row by id in a lock-compatible path and convert it to response info.
#[instrument(level = "info", skip_all)]
async fn get_info_excluded(conn: &mut RdbConn, id: &str) -> BaseRest<TermInfo> {
    //
    // Use `for_update()` to prevent concurrent updates while resolving this term.
    let row = t_term
        .filter(f_id.eq(id))
        .select(TermInfoRow::as_select())
        .for_update()
        .get_result::<TermInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
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
    };

    accept(row.into())
}

// Build a filtered query for term listing and execute a paged query.
#[instrument(level = "info", skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    termbase_id: &str,
    fuzzy_source: Option<&str>,
    offset: u32,
    limit: u32,
) -> BaseRest<Vec<TermInfo>> {
    //
    // Start with a termbase constraint, then apply optional fuzzy source matching.
    let mut query = t_term
        .filter(f_termbase_id.eq(termbase_id))
        .select(TermInfoRow::as_select())
        .into_boxed();

    if let Some(fuzzy_source) = fuzzy_source {
        //
        let escaped = escape_ilike_pattern(fuzzy_source);

        let pattern = format!("%{}%", escaped);

        query = query.filter(f_source.ilike(pattern));
    }

    let rows = query
        .order_by(f_updated_at.desc())
        .offset(offset as i64)
        .limit(limit as i64)
        .load::<TermInfoRow>(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

// Load one term row by id and convert it into response info.
#[instrument(level = "info", skip_all)]
async fn get_info(conn: &mut RdbConn, id: &str) -> BaseRest<TermInfo> {
    //
    // Read the row with strict id match and convert it to a rich term view.
    let row = t_term
        .filter(f_id.eq(id))
        .select(TermInfoRow::as_select())
        .get_result::<TermInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
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
    };

    accept(row.into())
}
