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
    TeamInfoRecord, TeamItem, TeamPack,
};
use crate::part_impl::repo::rdb_impl::numeric::usize_from_i32;
use crate::part_impl::repo::rdb_impl::schema::t_member;
use crate::part_impl::repo::rdb_impl::schema::t_team::dsl::{
    f_created_at, f_deleted_at, f_id, f_workset_next_index, t_team,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;

/// Build the expected error for a missing team.
pub fn missing_team(id: &str, operation: &str) -> BaseError {
    //
    expected_error! {
        ExpectedVariant::Args,
        message: trl("error-team-not-found");
        team_id = %id,
        operation,
        "team operation rejected",
    }
}

// Insert a team domain entry and return the persisted team info.
/// Insert a new team record from a domain entry.
#[instrument(level = "info", skip_all)]
pub async fn create(
    conn: &mut RdbConn,
    entry: &TeamEntry,
) -> BaseRest<TeamInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let item = TeamItem {
        f_id: &entry.id,
        f_name: entry.trimmed_name(),
        f_description: entry.trimmed_description(),
        f_workset_next_index: 0,
        f_created_at: now,
        f_updated_at: now,
    };

    let record = diesel::insert_into(t_team)
        .values(&item)
        .returning(TeamInfoRecord::as_returning())
        .get_result::<TeamInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    record.try_into()
}

// Load one team by id and convert it into DTO view model.
/// Load a single team info by ID.
#[instrument(level = "info", skip_all)]
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<TeamInfo> {
    //
    let record = t_team
        .filter(f_id.eq(id))
        .filter(f_deleted_at.is_null())
        .select(TeamInfoRecord::as_select())
        .get_result::<TeamInfoRecord>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(record) = record else {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-team-not-found");
            error_variant = ?ExpectedVariant::Args,
            team_id = %id,
            operation = "get team info",
            "team operation rejected",
        });
    };

    record.try_into()
}

// Query teams using an optional membership filter, ordering and pagination.
/// List team infos filtered and paginated by spec.
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    conn: &mut RdbConn,
    spec: &TeamListSpec,
) -> BaseRest<Vec<TeamInfo>> {
    //
    let mut query = t_team.filter(f_deleted_at.is_null()).into_boxed();

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

    let records = query
        .select(TeamInfoRecord::as_select())
        .order_by(f_created_at.desc())
        .offset(i64::from(spec.offset))
        .limit(i64::from(spec.limit.get()))
        .load::<TeamInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    records.into_iter().map(TryInto::try_into).collect()
}

// Update mutable team profile fields for the target team.
/// Apply a team metadata replacement.
#[instrument(level = "info", skip_all)]
pub async fn update_info(conn: &mut RdbConn, repl: &TeamRepl) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let pack = TeamPack::new(now)
        .name(repl.trimmed_name())
        .description(repl.trimmed_description());

    let updated_count = diesel::update(
        t_team
            .filter(f_id.eq(&repl.id))
            .filter(f_deleted_at.is_null()),
    )
    .set(&pack)
    .execute(conn)
    .await
    .map_err(diesel)?;

    if updated_count == 0 {
        return Err(missing_team(&repl.id, "update team info"));
    }

    accept(())
}

// Load one team info and lock the record for transactional updates.
/// Load a team info by ID, locking the record for update.
#[instrument(level = "info", skip_all)]
pub async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<TeamInfo> {
    //
    let record = t_team
        .filter(f_id.eq(id))
        .filter(f_deleted_at.is_null())
        .select(TeamInfoRecord::as_select())
        .for_update()
        .get_result::<TeamInfoRecord>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(record) = record else {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-team-not-found");
            error_variant = ?ExpectedVariant::Args,
            team_id = %id,
            operation = "lock team info",
            "team operation rejected",
        });
    };

    record.try_into()
}

// Lock a team record to serialize concurrent writes in the current transaction.
/// Lock a team record for a transactional update.
#[instrument(level = "info", skip_all)]
pub async fn lock_team(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    let record = t_team
        .filter(f_id.eq(id))
        .filter(f_deleted_at.is_null())
        .select(f_id)
        .for_update()
        .get_result::<String>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(_) = record else {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-team-not-found");
            error_variant = ?ExpectedVariant::Args,
            team_id = %id,
            operation = "lock team record",
            "team operation rejected",
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
    let prev = diesel::update(
        t_team.filter(f_id.eq(id)).filter(f_deleted_at.is_null()),
    )
    .set(f_workset_next_index.eq(f_workset_next_index + 1))
    .returning(f_workset_next_index - 1)
    .get_result::<i32>(conn)
    .await
    .optional()
    .map_err(diesel)?;

    let Some(prev) = prev else {
        return Err(missing_team(id, "allocate team workset index"));
    };

    accept(usize_from_i32(prev, "t_team.f_workset_next_index")?)
}
