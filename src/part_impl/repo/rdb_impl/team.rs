//! RDB-backed team repository — free query functions and thin trait impls.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_orchestra::{Run, Step};

use crate::complex::team::TeamComplex;
use crate::model::team::{TeamAvatarReservation,TeamEntry,TeamInfo};
use crate::part::repo::oper::team::{
    AllocateTeamWorksetIndex, CreateTeam, DeleteTeam, GetTeamInfo,
    GetTeamInfoExcluded, ListTeamInfos, ReserveTeamAvatar, UpdateTeam,
};
use crate::part::repo::team::TeamRepo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::team::{
    TeamAspect, TeamRow, TeamRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_member;
use crate::part_impl::repo::rdb_impl::schema::t_team::dsl::*;
use crate::part_impl::shared::result::{diesel, expected, version};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{RegularError, RegularResult};

impl TeamRepo<RdbContext> for RdbRepo {}

// ── Free functions ──────────────────────────────────────────────────────────

/// Insert a new team and return its info.
async fn create(
    conn: &mut RdbConn,
    entry: &TeamEntry,
) -> RegularResult<TeamInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let entry = TeamRowEntry {
        f_id: &entry.id,
        f_name: &entry.name,
        f_description: &entry.description,
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
) -> RegularResult<TeamInfo> {
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
) -> RegularResult<Vec<TeamInfo>> {
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
) -> RegularResult<TeamAvatarReservation> {
    //
    let now = OffsetDateTime::now_utc();

    let (prev_key, raw_version): (Option<String>, i64) =
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

    let version = version(raw_version)?;

    let object_key = TeamComplex::gen_avatar_key(id, version, file_ext);

    diesel::update(t_team.filter(f_id.eq(id)))
        .set((f_avatar_key.eq(Some(&object_key)), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(TeamAvatarReservation {
        object_key,
        prev_object_key: prev_key,
        avatar_version: version,
    })
}

/// Load a team info by ID, locking the row for update.
async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<TeamInfo> {
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

impl<'a> Run<CreateTeam<'a>> for RdbRepo {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &CreateTeam<'a>,
    ) -> Result<TeamInfo, Self::Error> {
        submit_query!(self.core, create, oper.entry)
    }
}

impl<'a> Run<GetTeamInfo<'a>> for RdbRepo {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &GetTeamInfo<'a>,
    ) -> Result<TeamInfo, Self::Error> {
        match oper {
            GetTeamInfo::Id { id } => {
                submit_query!(self.core, get_info_by_id, id)
            }
        }
    }
}

impl<'a> Run<ListTeamInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &ListTeamInfos<'a>,
    ) -> Result<Vec<TeamInfo>, Self::Error> {
        submit_query!(
            self.core,
            list_infos,
            oper.user_id,
            oper.offset,
            oper.limit
        )
    }
}

impl<'a> Run<UpdateTeam<'a>> for RdbRepo {
    type Error = RegularError;

    async fn run(&self, oper: &UpdateTeam<'a>) -> RegularResult<()> {
        match oper {
            //
            UpdateTeam::Info {
                id,
                name,
                description,
            } => submit_query!(self.core, update_info, id, name, description),

            UpdateTeam::MarkAvatarUploaded { id, avatar_version } => {
                submit_query!(
                    self.core,
                    mark_avatar_uploaded,
                    id,
                    *avatar_version
                )
            }
        }
    }
}

impl<'a> Step<CreateTeam<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateTeam<'a>,
    ) -> RegularResult<TeamInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl<'a> Step<UpdateTeam<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateTeam<'a>,
    ) -> RegularResult<()> {
        match oper {
            //
            UpdateTeam::Info {
                id,
                name,
                description,
            } => update_info(context.conn(), id, name, description).await,

            UpdateTeam::MarkAvatarUploaded { id, avatar_version } => {
                mark_avatar_uploaded(context.conn(), id, *avatar_version).await
            }
        }
    }
}

impl<'a> Step<ReserveTeamAvatar<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ReserveTeamAvatar<'a>,
    ) -> RegularResult<TeamAvatarReservation> {
        reserve_avatar(context.conn(), oper.id, oper.file_ext).await
    }
}

impl<'a> Step<GetTeamInfoExcluded<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetTeamInfoExcluded<'a>,
    ) -> RegularResult<TeamInfo> {
        match oper {
            GetTeamInfoExcluded::Id { id } => {
                get_info_excluded(context.conn(), id).await
            }
        }
    }
}

impl<'a> Step<DeleteTeam<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteTeam<'a>,
    ) -> RegularResult<()> {
        delete(context.conn(), oper.id).await
    }
}

impl<'a> Step<AllocateTeamWorksetIndex<'a>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &AllocateTeamWorksetIndex<'a>,
    ) -> RegularResult<i32> {
        increment_workset_next_index(context.conn(), oper.id).await
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;
