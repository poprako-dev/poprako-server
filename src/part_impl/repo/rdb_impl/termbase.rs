//! Diesel-backed terminology-base repository operations.

/// Termbase RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use diesel::{
    BoolExpressionMethods as _, ExpressionMethods as _,
    NullableExpressionMethods as _, OptionalExtension as _,
    PgTextExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::termbase::TermbaseInfo;
use crate::model::read::spec::termbase::TermbaseListSpec;
use crate::model::write::termbase::{TermbaseEntry, TermbaseRepl};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::termbase::{
    CreateTermbase, DeleteTermbase, GetTermbaseInfo, GetTermbaseInfoExcluded,
    ListTermbaseInfos, ListTermbaseInfosExcluded, TouchTermbase,
    UpdateTermbase, UpdateTermbaseTermCount,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::termbase::{
    TermbaseInfoRecord, TermbaseItem,
};
use crate::part_impl::repo::rdb_impl::schema::t_termbase::dsl::{
    f_comic_id, f_description, f_id, f_name, f_team_id, f_term_count,
    f_updated_at, t_termbase,
};
use crate::part_impl::repo::rdb_impl::schema::{t_comic, t_workset};
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::shared::RdbContext;
use crate::shared::result::diesel;

// Escape `%` and `_` wildcard symbols so fuzzy search stays literal-safe.
// Remove one termbase record by id.
#[instrument(level = "info", skip_all)]
async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    // Execute hard delete as the finalization action in repositories.
    diesel::delete(t_termbase.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept!(())
}

// Escape wildcard and escape characters before constructing ILIKE patterns.
fn escape_ilike_pattern(input: &str) -> String {
    //
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

// Load one termbase record by id and map DB entity into response shape.
#[instrument(level = "info", skip_all)]
async fn get_info(conn: &mut RdbConn, id: &str) -> BaseRest<TermbaseInfo> {
    //
    // Return explicit not-found error when the target termbase does not exist.
    let record = t_termbase
        .filter(f_id.eq(id))
        .select(TermbaseInfoRecord::as_select())
        .get_result::<TermbaseInfoRecord>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(record) = record else {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-termbase-not-found");
            error_variant = ?ExpectedVariant::Args,
            termbase_id = %id,
            operation = "get termbase info",
            "termbase operation rejected",
        });
    };

    accept!(TermbaseInfo::try_from(record)?)
}

// Load one termbase record by id with record lock for transactional mutation.
#[instrument(level = "info", skip_all)]
async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<TermbaseInfo> {
    //
    // Take `FOR UPDATE` lock and keep semantics aligned with locked read paths.
    let record = t_termbase
        .filter(f_id.eq(id))
        .select(TermbaseInfoRecord::as_select())
        .for_update()
        .get_result::<TermbaseInfoRecord>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(record) = record else {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-termbase-not-found");
            error_variant = ?ExpectedVariant::Args,
            termbase_id = %id,
            operation = "lock termbase info",
            "termbase operation rejected",
        });
    };

    accept!(TermbaseInfo::try_from(record)?)
}

// List termbase records with team/comic filter and optional fuzzy name.
#[instrument(level = "info", skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &TermbaseListSpec,
) -> BaseRest<Vec<TermbaseInfo>> {
    //
    // Build one query path that branches on scope and optional fuzzy name.
    let mut query = t_termbase
        .select(TermbaseInfoRecord::as_select())
        .into_boxed();

    let (offset, limit) = match spec {
        //
        TermbaseListSpec::Team {
            team_id,
            fuzzy_name: _,
            offset,
            limit,
        } => {
            //
            query = query.filter(f_team_id.eq(team_id));

            (offset, limit)
        }

        TermbaseListSpec::Comic {
            comic_id,
            fuzzy_name: _,
            offset,
            limit,
        } => {
            //
            let owning_team_ids = t_comic::table
                .inner_join(t_workset::table)
                .filter(t_comic::f_id.eq(comic_id))
                .select(t_workset::f_team_id.nullable());

            query = query.filter(
                f_team_id
                    .eq_any(owning_team_ids)
                    .or(f_comic_id.eq(comic_id)),
            );

            (offset, limit)
        }
    };

    if let Some(fuzzy_name) = spec.trimmed_fuzzy_name() {
        //
        let escaped = escape_ilike_pattern(fuzzy_name);

        let pattern = ["%", escaped.as_str(), "%"].concat();

        query = query.filter(f_name.ilike(pattern));
    }

    let records = query
        .order_by(f_updated_at.desc())
        .offset(i64::from(*offset))
        .limit(i64::from(limit.get()))
        .load::<TermbaseInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    records.into_iter().map(TermbaseInfo::try_from).collect()
}

// List all termbases for team/comic with record lock for later mutation.
#[instrument(level = "info", skip_all)]
async fn list_infos_excluded(
    conn: &mut RdbConn,
    oper: &ListTermbaseInfosExcluded<'_>,
) -> BaseRest<Vec<TermbaseInfo>> {
    //
    // Lock cand records so subsequent writes in caller transaction stay safe.
    let records = match oper {
        //
        ListTermbaseInfosExcluded::Team { team_id } => t_termbase
            .filter(f_team_id.eq(team_id))
            .select(TermbaseInfoRecord::as_select())
            .for_update()
            .load::<TermbaseInfoRecord>(conn)
            .await
            .map_err(diesel)?,

        ListTermbaseInfosExcluded::Comic { comic_id } => t_termbase
            .filter(f_comic_id.eq(comic_id))
            .select(TermbaseInfoRecord::as_select())
            .for_update()
            .load::<TermbaseInfoRecord>(conn)
            .await
            .map_err(diesel)?,
    };

    records.into_iter().map(TermbaseInfo::try_from).collect()
}

// Insert a new termbase and return created info.
#[instrument(level = "info", skip_all)]
async fn create(
    conn: &mut RdbConn,
    entry: &TermbaseEntry,
) -> BaseRest<TermbaseInfo> {
    //
    // Convert API entry into insert record shape and fetch persisted record.
    let item = TermbaseItem::from(entry);

    let record = diesel::insert_into(t_termbase)
        .values(&item)
        .returning(TermbaseInfoRecord::as_returning())
        .get_result::<TermbaseInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    accept!(TermbaseInfo::try_from(record)?)
}

// Update termbase descriptive fields.
#[instrument(level = "info", skip_all)]
async fn update_info(
    conn: &mut RdbConn,
    update: &TermbaseRepl,
) -> BaseRest<()> {
    //
    // Keep `updated_at` current while applying partial name/description updates.
    diesel::update(t_termbase.filter(f_id.eq(&update.id)))
        .set((
            f_name.eq(update.trimmed_name()),
            f_description.eq(update.trimmed_description()),
            f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept!(())
}

// Adjust term count atomically by signed delta.
#[instrument(level = "info", skip_all)]
async fn update_term_count(
    conn: &mut RdbConn,
    id: &str,
    delta: i32,
) -> BaseRest<()> {
    //
    // Use SQL delta update to avoid read-modify-write races.
    diesel::update(t_termbase.filter(f_id.eq(id)))
        .set((
            f_term_count.eq(f_term_count + delta),
            f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept!(())
}

// Touch updated_at to indicate activity without changing business fields.
#[instrument(level = "info", skip_all)]
async fn touch(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    // Keep liveness and stale-checking aligned for external schedulers.
    diesel::update(t_termbase.filter(f_id.eq(id)))
        .set(f_updated_at.eq(OffsetDateTime::now_utc()))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept!(())
}

impl Run<GetTermbaseInfo<'_>> for HybRepo {
    // Use BaseError for non-transactional read failures.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Load one termbase info by id through shared query path.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &GetTermbaseInfo<'_>) -> BaseRest<TermbaseInfo> {
        submit_query!(self.rdb_core, get_info, oper.id)
    }
}

impl Run<ListTermbaseInfos<'_>> for HybRepo {
    // Use BaseError for non-transactional list failures.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Load paged termbase list by spec through shared query path.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ListTermbaseInfos<'_>,
    ) -> BaseRest<Vec<TermbaseInfo>> {
        submit_query!(self.rdb_core, list_infos, oper.spec)
    }
}

impl<L> Step<CreateTermbase<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use BaseError for transactional creation failures.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Insert termbase record inside transaction context and return persisted info.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateTermbase<'_>,
    ) -> BaseRest<TermbaseInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<L> Step<GetTermbaseInfo<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use BaseError for transactional read failures.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Read one locked? or unlocked termbase info in step context.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetTermbaseInfo<'_>,
    ) -> BaseRest<TermbaseInfo> {
        get_info(context.conn(), oper.id).await
    }
}

impl<L> Step<GetTermbaseInfoExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use BaseError for transactional lock-bound reads.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Read one termbase record with `FOR UPDATE` for subsequent writes.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &GetTermbaseInfoExcluded<'_>,
    ) -> BaseRest<TermbaseInfo> {
        get_info_excluded(context.conn(), oper.id).await
    }
}

impl<L> Step<ListTermbaseInfosExcluded<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use BaseError for transactional collection reads.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Read and lock all records for a team/comic scope.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ListTermbaseInfosExcluded<'_>,
    ) -> BaseRest<Vec<TermbaseInfo>> {
        list_infos_excluded(context.conn(), oper).await
    }
}

impl<L> Step<UpdateTermbase<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use BaseError for transactional update failures.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Apply name/description changes in one update statement.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdateTermbase<'_>,
    ) -> BaseRest<()> {
        update_info(context.conn(), oper.update).await
    }
}

impl<L> Step<UpdateTermbaseTermCount<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use BaseError for transactional aggregate count updates.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Update term count with delta while keeping updated_at fresh.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &UpdateTermbaseTermCount<'_>,
    ) -> BaseRest<()> {
        update_term_count(context.conn(), oper.id, oper.delta).await
    }
}

impl<L> Step<TouchTermbase<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use BaseError for transactional touch/update timestamp failures.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Refresh termbase updated_at in transactional flow.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &TouchTermbase<'_>,
    ) -> BaseRest<()> {
        touch(context.conn(), oper.id).await
    }
}

impl<L> Step<DeleteTermbase<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Use BaseError for transactional deletion failures.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Delete termbase record as part of transaction flow.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &DeleteTermbase<'_>,
    ) -> BaseRest<()> {
        delete(context.conn(), oper.id).await
    }
}
