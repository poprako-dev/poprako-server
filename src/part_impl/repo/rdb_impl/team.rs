//! RDB-backed team repository — free query functions and thin trait impls.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::complex::team::TeamComplex;
use crate::model::team_model;
use crate::part::repo::step::team::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrementWorksetNextIndex,
    ListInfos, MarkAvatarUploaded, ReserveAvatar, UpdateInfo,
};
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::rdb_impl::entity::team::{
    TeamAspect, TeamEntry, TeamRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_member;
use crate::part_impl::repo::rdb_impl::schema::t_team::dsl::*;
use crate::part_impl::repo::rdb_impl::{RdbRepo, RdbRepoTransactional};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{RegularError, RegularResult};

impl TeamRepo<RdbContext> for RdbRepo {}

impl TeamRepoTransactional<RdbContext> for RdbRepoTransactional {}

// ── Free functions ──────────────────────────────────────────────────────────

/// Insert a new team and return its info.
async fn create(
    conn: &mut RdbConn,
    form: &team_model::Form,
) -> RegularResult<team_model::Info> {
    //
    let now = OffsetDateTime::now_utc();

    let entry = TeamEntry {
        f_id: &form.id,
        f_name: &form.name,
        f_description: &form.description,
        f_workset_next_index: 0,
        f_created_at: now,
        f_updated_at: now,
    };

    let row: TeamRow = diesel::insert_into(t_team)
        .values(&entry)
        .returning(TeamRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(row.into())
}

/// Load a single team info by ID.
async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<team_model::Info> {
    //
    let row: TeamRow = t_team
        .filter(f_id.eq(id))
        .select(TeamRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-team-not-found"))?;

    Ok(row.into())
}

/// Query teams, optionally filtered to those a user is a member of.
async fn list_infos(
    conn: &mut RdbConn,
    user_id: Option<&str>,
    offset: u32,
    limit: u32,
) -> RegularResult<Vec<team_model::Info>> {
    //
    let mut query = t_team.into_boxed();

    if let Some(user_id) = user_id {
        //
        let member_team_ids = t_member::table
            .filter(t_member::f_user_id.eq(user_id))
            .select(t_member::f_team_id);

        query = query.filter(f_id.eq_any(member_team_ids));
    }

    let rows: Vec<TeamRow> = query
        .select(TeamRow::as_select())
        .order_by(f_created_at.desc())
        .offset(offset as i64)
        .limit(limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Update a team's name and description.
async fn update_info(
    conn: &mut RdbConn,
    id: &str,
    name: &str,
    description: &str,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = TeamAspect::new(now).name(name).description(description);

    diesel::update(t_team.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Mark a team avatar as uploaded, checking version staleness.
async fn mark_avatar_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: u32,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let affected = diesel::update(
        t_team
            .filter(f_id.eq(id))
            .filter(f_avatar_version.eq(i64::from(version))),
    )
    .set((f_avatar_uploaded.eq(true), f_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    if affected == 0 {
        return Err(expected("error-avatar-version-mismatch"));
    }

    Ok(())
}

/// Reserve a new avatar slot for a team: bump version, generate object key,
/// and return the reservation with previous key for cleanup.
async fn reserve_avatar(
    conn: &mut RdbConn,
    id: &str,
    file_ext: &str,
) -> RegularResult<team_model::AvatarReservation> {
    //
    let now = OffsetDateTime::now_utc();

    let (prev_key, version): (Option<String>, i64) =
        diesel::update(t_team.filter(f_id.eq(id)))
            .set((
                f_avatar_key.eq::<Option<&str>>(None),
                f_avatar_uploaded.eq(false),
                f_avatar_version.eq(f_avatar_version + 1),
                f_updated_at.eq(now),
            ))
            .returning((f_avatar_key, f_avatar_version))
            .get_result(conn)
            .await
            .map_err(diesel)?;

    let version = crate::part_impl::shared::result::version(version)?;

    let object_key = TeamComplex::gen_avatar_key(id, version, file_ext);

    diesel::update(t_team.filter(f_id.eq(id)))
        .set((f_avatar_key.eq(Some(&object_key)), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(team_model::AvatarReservation {
        object_key,
        prev_object_key: prev_key,
        avatar_version: version,
    })
}

/// Load a team info by ID, locking the row for update.
async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<team_model::Info> {
    //
    let row: TeamRow = t_team
        .filter(f_id.eq(id))
        .select(TeamRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-team-not-found"))?;

    Ok(row.into())
}

/// Delete a team by ID.
async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    //
    diesel::delete(t_team.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Atomically increment and return the previous workset-next-index for a team.
async fn increment_workset_next_index(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<i32> {
    //
    let prev: i32 = diesel::update(t_team.filter(f_id.eq(id)))
        .set(f_workset_next_index.eq(f_workset_next_index + 1))
        .returning(f_workset_next_index - 1)
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(prev)
}

// ── Non-transactional: Execute impls ─────────────────────────────────────────

#[async_trait]
impl<'a> Execute<Create<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &Create<'a>,
    ) -> Result<team_model::Info, Self::Error> {
        submit_query!(self.core, create, step.form)
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> Result<team_model::Info, Self::Error> {
        submit_query!(self.core, get_info_by_id, step.id)
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> Result<Vec<team_model::Info>, Self::Error> {
        submit_query!(
            self.core,
            list_infos,
            step.user_id,
            step.offset,
            step.limit
        )
    }
}

#[async_trait]
impl<'a> Execute<UpdateInfo<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &UpdateInfo<'a>) -> RegularResult<()> {
        submit_query!(
            self.core,
            update_info,
            step.id,
            step.name,
            step.description
        )
    }
}

#[async_trait]
impl<'a> Execute<MarkAvatarUploaded<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &MarkAvatarUploaded<'a>,
    ) -> RegularResult<()> {
        submit_query!(
            self.core,
            mark_avatar_uploaded,
            step.id,
            step.avatar_version
        )
    }
}

// ── Transactional: Advance impls ─────────────────────────────────────────────

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Create<'a>,
    ) -> RegularResult<team_model::Info> {
        create(context.conn(), step.form).await
    }
}

#[async_trait]
impl<'a> Advance<ReserveAvatar<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ReserveAvatar<'a>,
    ) -> RegularResult<team_model::AvatarReservation> {
        reserve_avatar(context.conn(), step.id, step.file_extension).await
    }
}

#[async_trait]
impl<'a> Advance<MarkAvatarUploaded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &MarkAvatarUploaded<'a>,
    ) -> RegularResult<()> {
        mark_avatar_uploaded(context.conn(), step.id, step.avatar_version).await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoExcluded<'a>,
    ) -> RegularResult<team_model::Info> {
        get_info_excluded(context.conn(), step.id).await
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Delete<'a>,
    ) -> RegularResult<()> {
        delete(context.conn(), step.id).await
    }
}

#[async_trait]
impl<'a> Advance<IncrementWorksetNextIndex<'a>, RdbContext>
    for RdbRepoTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &IncrementWorksetNextIndex<'a>,
    ) -> RegularResult<i32> {
        increment_workset_next_index(context.conn(), step.id).await
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;
