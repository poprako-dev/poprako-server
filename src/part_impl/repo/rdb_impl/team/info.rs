//! Team lifecycle and profile persistence.

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::team::TeamInfo;
use crate::model::read::spec::team::TeamListSpec;
use crate::model::write::team::{TeamEntry, TeamRepl};
use crate::part_impl::repo::rdb_impl::entity::team::{
    TeamAspectRow, TeamEntryRow, TeamInfoRow,
};
use crate::part_impl::repo::rdb_impl::numeric::usize_from_i32;
use crate::part_impl::repo::rdb_impl::schema::t_member;
use crate::part_impl::repo::rdb_impl::schema::t_team::dsl::{
    f_created_at, f_id, f_workset_next_index, t_team,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;

/// Delete a team row by primary key.
pub async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    diesel::delete(t_team.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Insert a team entry and return the persisted team info.
/// Insert a new team row from an entry.
#[instrument(level = "info", skip_all)]
pub async fn create(
    conn: &mut RdbConn,
    entry: &TeamEntry,
) -> BaseRest<TeamInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let entry = TeamEntryRow {
        f_id: &entry.id,
        f_name: &entry.name,
        f_description: &entry.description,
        f_workset_next_index: 0,
        f_created_at: now,
        f_updated_at: now,
    };

    let row = diesel::insert_into(t_team)
        .values(&entry)
        .returning(TeamInfoRow::as_returning())
        .get_result::<TeamInfoRow>(conn)
        .await
        .map_err(diesel)?;

    row.try_into()
}

// Load one team by id and convert it into DTO view model.
/// Load a single team info by ID.
#[instrument(level = "info", skip_all)]
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<TeamInfo> {
    //
    let row = t_team
        .filter(f_id.eq(id))
        .select(TeamInfoRow::as_select())
        .get_result::<TeamInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        //
        let message = trl("error-team-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            team_id = %id,
            operation = "get team info",
            "expected team error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    };

    row.try_into()
}

// Query teams using an optional membership filter, ordering and pagination.
/// List team infos filtered and paginated by spec.
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    conn: &mut RdbConn,
    spec: &TeamListSpec,
) -> BaseRest<Vec<TeamInfo>> {
    //
    let mut query = t_team.into_boxed();

    query = match spec.user_id.as_deref() {
        //
        Some(user_id) => {
            //
            let member_team_ids = t_member::table
                .filter(t_member::f_user_id.eq(user_id))
                .select(t_member::f_team_id);

            query.filter(f_id.eq_any(member_team_ids))
        }

        None => query,
    };

    let rows = query
        .select(TeamInfoRow::as_select())
        .order_by(f_created_at.desc())
        .offset(i64::from(spec.offset))
        .limit(i64::from(spec.limit))
        .load::<TeamInfoRow>(conn)
        .await
        .map_err(diesel)?;

    rows.into_iter().map(TryInto::try_into).collect()
}

// Update mutable team profile fields for the target team.
/// Apply a team metadata replacement.
#[instrument(level = "info", skip_all)]
pub async fn update_info(conn: &mut RdbConn, repl: &TeamRepl) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = TeamAspectRow::new(now)
        .name(&repl.name)
        .description(&repl.description);

    diesel::update(t_team.filter(f_id.eq(&repl.id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Load one team info and lock the row for transactional updates.
/// Load a team info by ID, locking the row for update.
#[instrument(level = "info", skip_all)]
pub async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<TeamInfo> {
    //
    let row = t_team
        .filter(f_id.eq(id))
        .select(TeamInfoRow::as_select())
        .for_update()
        .get_result::<TeamInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        //
        let message = trl("error-team-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            team_id = %id,
            operation = "lock team info",
            "expected team error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    };

    row.try_into()
}

// Lock a team row to serialize concurrent writes in the current transaction.
/// Lock a team row for a transactional update.
#[instrument(level = "info", skip_all)]
pub async fn lock_team(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    let row = t_team
        .filter(f_id.eq(id))
        .select(f_id)
        .for_update()
        .get_result::<String>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(_) = row else {
        //
        let message = trl("error-team-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            team_id = %id,
            operation = "lock team row",
            "expected team error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    };

    accept(())
}

// Advance workset sequence and return previous value for deterministic IDs.
/// Increment and return the workset next index.
#[instrument(level = "info", skip_all)]
pub async fn increment_workset_next_index(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<usize> {
    //
    let prev = diesel::update(t_team.filter(f_id.eq(id)))
        .set(f_workset_next_index.eq(f_workset_next_index + 1))
        .returning(f_workset_next_index - 1)
        .get_result::<i32>(conn)
        .await
        .map_err(diesel)?;

    accept(usize_from_i32(prev, "t_team.f_workset_next_index")?)
}
