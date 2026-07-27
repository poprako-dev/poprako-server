//! RDB-backed team repository — free query functions and thin trait impls.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use crate::complex::team::TeamComplex;
use crate::model::team::{
    TeamAvatarReservation, TeamEntry, TeamInfo, TeamInfoListKind,
    TeamInfoListSpec,
};
use crate::part::repo::oper::team::{
    AllocTeamWorksetIndex, CreateTeam, DeleteTeam, GetTeamInfo,
    GetTeamInfoExcluded, ListTeamInfos, LockTeam, ReserveTeamAvatar,
    UpdateTeam,
};
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::repo::rdb_impl::entity::team::{
    TeamAspect, TeamRow, TeamRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_member;
use crate::part_impl::repo::rdb_impl::schema::t_team::dsl::*;
use crate::part_impl::shared::result::{diesel, expected, next_version};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{BaseError, BaseResult, accept};
use crate::value::image::{ImageExt, ImageHash};

/// Team RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

// ── Free functions ──────────────────────────────────────────────────────────

/// Insert a new team and return its info.
#[instrument(level = "info", err(Debug), skip_all)]
async fn create(conn: &mut RdbConn, entry: &TeamEntry) -> BaseResult<TeamInfo> {
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

    row.try_into()
}

/// Load a single team info by ID.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_by_id(conn: &mut RdbConn, id: &str) -> BaseResult<TeamInfo> {
    //
    let row: TeamRow = t_team
        .filter(f_id.eq(id))
        .select(TeamRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-team-not-found"))?;

    row.try_into()
}

/// Query teams selected by a list specification.
#[instrument(level = "info", err(Debug), skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &TeamInfoListSpec,
) -> BaseResult<Vec<TeamInfo>> {
    //
    let mut query = t_team.into_boxed();

    query = match &spec.kind {
        //
        TeamInfoListKind::All => query,

        TeamInfoListKind::JoinedBy { user_id } => {
            //
            let member_team_ids = t_member::table
                .filter(t_member::f_user_id.eq(user_id))
                .select(t_member::f_team_id);

            query.filter(f_id.eq_any(member_team_ids))
        }
    };

    let rows: Vec<TeamRow> = query
        .select(TeamRow::as_select())
        .order_by(f_created_at.desc())
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    rows.into_iter().map(TryInto::try_into).collect()
}

/// Update a team's name and description.
#[instrument(level = "info", err(Debug), skip_all)]
async fn update_info(
    conn: &mut RdbConn,
    id: &str,
    name: &str,
    description: &str,
) -> BaseResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = TeamAspect::new(now).name(name).description(description);

    diesel::update(t_team.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Mark a team avatar as uploaded, checking version staleness.
#[instrument(level = "info", err(Debug), skip_all)]
async fn mark_avatar_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: u32,
    avatar_key: Option<&str>,
    avatar_uploaded: bool,
) -> BaseResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let affected = match avatar_key {
        //
        Some(avatar_key) => {
            diesel::update(
                t_team
                    .filter(f_id.eq(id))
                    .filter(f_avatar_version.eq(i64::from(version)))
                    .filter(f_avatar_key.eq(avatar_key)),
            )
            .set((f_avatar_uploaded.eq(avatar_uploaded), f_updated_at.eq(now)))
            .execute(conn)
            .await
        }

        None => {
            diesel::update(
                t_team
                    .filter(f_id.eq(id))
                    .filter(f_avatar_version.eq(i64::from(version))),
            )
            .set((f_avatar_uploaded.eq(avatar_uploaded), f_updated_at.eq(now)))
            .execute(conn)
            .await
        }
    }
    .map_err(diesel)?;

    if affected == 0 {
        return Err(expected("error-avatar-version-mismatch"));
    }

    accept(())
}

/// Reserve a new avatar slot for a team: bump version, generate object key,
/// and return the reservation with previous key for cleanup.
#[instrument(level = "info", err(Debug), skip_all)]
async fn reserve_avatar(
    conn: &mut RdbConn,
    id: &str,
    image_hash: &ImageHash,
    image_ext: ImageExt,
) -> BaseResult<TeamAvatarReservation> {
    //
    let now = OffsetDateTime::now_utc();

    let (prev_key, uploaded, raw_version, stored_hash, stored_ext): (
        Option<String>,
        bool,
        i64,
        Vec<u8>,
        String,
    ) = t_team
        .filter(f_id.eq(id))
        .select((
            f_avatar_key,
            f_avatar_uploaded,
            f_avatar_version,
            f_avatar_hash,
            f_avatar_extension,
        ))
        .for_update()
        .get_result(conn)
        .await
        .map_err(diesel)?;

    let same_hash =
        prev_key.is_some() && stored_hash.as_slice() == image_hash.as_bytes();

    if same_hash && stored_ext != image_ext.suffix() {
        return Err(expected("error-invalid-image-extension"));
    }

    if same_hash {
        //
        let object_key = prev_key.ok_or_else(|| BaseError::Unrecoverable {
            message: "[reserve_avatar] pending avatar key is missing".into(),
        })?;

        return accept(TeamAvatarReservation {
            object_key,
            prev_object_key: None,
            avatar_version: u32::try_from(raw_version).map_err(|_| {
                BaseError::Unrecoverable {
                    message: "[reserve_avatar] avatar version is invalid"
                        .into(),
                }
            })?,
            upload_required: !uploaded,
        });
    }

    let version = next_version(raw_version)?;

    let object_key =
        TeamComplex::gen_avatar_key(id, version, image_ext.suffix());

    diesel::update(t_team.filter(f_id.eq(id)))
        .set((
            f_avatar_key.eq(Some(&object_key)),
            f_avatar_uploaded.eq(false),
            f_avatar_version.eq(i64::from(version)),
            f_avatar_hash.eq(image_hash.as_bytes().to_vec()),
            f_avatar_extension.eq(image_ext.suffix()),
            f_updated_at.eq(now),
        ))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(TeamAvatarReservation {
        object_key,
        prev_object_key: prev_key,
        avatar_version: version,
        upload_required: true,
    })
}

/// Load a team info by ID, locking the row for update.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> BaseResult<TeamInfo> {
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

    row.try_into()
}

/// Locks a team row.
#[instrument(level = "info", err(Debug), skip_all)]
async fn lock_team(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    //
    let _: String = t_team
        .filter(f_id.eq(id))
        .select(f_id)
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-team-not-found"))?;

    accept(())
}

/// Delete a team by ID.
#[instrument(level = "info", err(Debug), skip_all)]
async fn delete(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    //
    diesel::delete(t_team.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Atomically increment and return the previous workset-next-index for a team.
#[instrument(level = "info", err(Debug), skip_all)]
async fn increment_workset_next_index(
    conn: &mut RdbConn,
    id: &str,
) -> BaseResult<i32> {
    //
    let prev: i32 = diesel::update(t_team.filter(f_id.eq(id)))
        .set(f_workset_next_index.eq(f_workset_next_index + 1))
        .returning(f_workset_next_index - 1)
        .get_result(conn)
        .await
        .map_err(diesel)?;

    accept(prev)
}

impl<'a> Run<CreateTeam<'a>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &CreateTeam<'_>,
    ) -> Result<TeamInfo, Self::Error> {
        submit_query!(self.core, create, oper.entry)
    }
}

impl Run<GetTeamInfo<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetTeamInfo<'_>,
    ) -> Result<TeamInfo, Self::Error> {
        match oper {
            GetTeamInfo::Id { id } => {
                submit_query!(self.core, get_info_by_id, id)
            }
        }
    }
}

impl Run<ListTeamInfos<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListTeamInfos<'_>,
    ) -> Result<Vec<TeamInfo>, Self::Error> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl Run<UpdateTeam<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &UpdateTeam<'_>) -> BaseResult<()> {
        match oper {
            //
            UpdateTeam::Info {
                id,
                name,
                description,
            } => submit_query!(self.core, update_info, id, name, description),

            UpdateTeam::MarkAvatarUploaded {
                id,
                avatar_version,
                avatar_key,
                avatar_uploaded,
            } => {
                submit_query!(
                    self.core,
                    mark_avatar_uploaded,
                    id,
                    *avatar_version,
                    *avatar_key,
                    *avatar_uploaded
                )
            }
        }
    }
}

impl Step<CreateTeam<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateTeam<'_>,
    ) -> BaseResult<TeamInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl Step<UpdateTeam<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateTeam<'_>,
    ) -> BaseResult<()> {
        match oper {
            //
            UpdateTeam::Info {
                id,
                name,
                description,
            } => update_info(context.conn(), id, name, description).await,

            UpdateTeam::MarkAvatarUploaded {
                id,
                avatar_version,
                avatar_key,
                avatar_uploaded,
            } => {
                mark_avatar_uploaded(
                    context.conn(),
                    id,
                    *avatar_version,
                    *avatar_key,
                    *avatar_uploaded,
                )
                .await
            }
        }
    }
}

impl Step<ReserveTeamAvatar<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ReserveTeamAvatar<'_>,
    ) -> BaseResult<TeamAvatarReservation> {
        reserve_avatar(context.conn(), oper.id, oper.image_hash, oper.image_ext)
            .await
    }
}

impl Step<GetTeamInfoExcluded<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetTeamInfoExcluded<'_>,
    ) -> BaseResult<TeamInfo> {
        match oper {
            GetTeamInfoExcluded::Id { id } => {
                get_info_excluded(context.conn(), id).await
            }
        }
    }
}

impl Step<LockTeam<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &LockTeam<'_>,
    ) -> BaseResult<()> {
        lock_team(context.conn(), oper.id).await
    }
}

impl Step<DeleteTeam<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteTeam<'_>,
    ) -> BaseResult<()> {
        delete(context.conn(), oper.id).await
    }
}

impl Step<AllocTeamWorksetIndex<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &AllocTeamWorksetIndex<'_>,
    ) -> BaseResult<i32> {
        increment_workset_next_index(context.conn(), oper.id).await
    }
}
