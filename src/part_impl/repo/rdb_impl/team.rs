//! RDB-backed team repository — free query functions and thin trait impls.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::team::TeamComplex;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::spec::team::TeamListSpec;
use crate::model::write::team::{TeamAvatarReservation, TeamEntry, TeamRepl};
use crate::part::repo::oper::team::{
    AllocTeamWorksetIndex, CreateTeam, DeleteTeam, GetTeamInfo,
    GetTeamInfoExcluded, ListTeamInfos, LockTeam, ReserveTeamAvatar,
    UpdateTeam,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::team::{
    TeamAspectRow, TeamEntryRow, TeamInfoRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_member;
use crate::part_impl::repo::rdb_impl::schema::t_team::dsl::*;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::{diesel, next_version};
use crate::shared::{RdbConn, RdbContext};
use crate::value::image::{ImageExt, ImageHash};

/// Team RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

// RDB team-ownership projections.
mod resolve;

// ── Free functions ──────────────────────────────────────────────────────────

// Delete a team row by primary key.
#[instrument(level = "info", skip_all)]
async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    diesel::delete(t_team.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Insert a team entry and return the persisted team info.
#[instrument(level = "info", skip_all)]
async fn create(conn: &mut RdbConn, entry: &TeamEntry) -> BaseRest<TeamInfo> {
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
#[instrument(level = "info", skip_all)]
async fn get_info_by_id(conn: &mut RdbConn, id: &str) -> BaseRest<TeamInfo> {
    //
    let row = t_team
        .filter(f_id.eq(id))
        .select(TeamInfoRow::as_select())
        .get_result::<TeamInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let row = match row {
        //
        Some(row) => row,

        None => {
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
        }
    };

    row.try_into()
}

// Query teams using an optional membership filter, ordering and pagination.
#[instrument(level = "info", skip_all)]
async fn list_infos(
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
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load::<TeamInfoRow>(conn)
        .await
        .map_err(diesel)?;

    rows.into_iter().map(TryInto::try_into).collect()
}

// Update mutable team profile fields for the target team.
#[instrument(level = "info", skip_all)]
async fn update_info(conn: &mut RdbConn, repl: &TeamRepl) -> BaseRest<()> {
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

// Validate version/hash preconditions and mark avatar upload state.
#[instrument(level = "info", skip_all)]
async fn mark_avatar_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: u32,
    avatar_key: Option<&str>,
    avatar_uploaded: bool,
) -> BaseRest<()> {
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
        //
        let message = trl("error-avatar-version-mismatch");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            team_id = %id,
            image_version = version,
            avatar_key = ?avatar_key,
            operation = "mark team avatar uploaded",
            "expected team avatar version error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    }

    accept(())
}

// Allocate a new avatar reservation version, returning object keys and cleanup metadata.
#[instrument(level = "info", skip_all)]
async fn reserve_avatar(
    conn: &mut RdbConn,
    id: &str,
    image_hash: &ImageHash,
    image_ext: ImageExt,
) -> BaseRest<TeamAvatarReservation> {
    //
    let now = OffsetDateTime::now_utc();

    let (prev_key, uploaded, raw_version, stored_hash, stored_ext) = t_team
        .filter(f_id.eq(id))
        .select((
            f_avatar_key,
            f_avatar_uploaded,
            f_avatar_version,
            f_avatar_hash,
            f_avatar_extension,
        ))
        .for_update()
        .get_result::<(
            Option<String>,
            Option<bool>,
            Option<i64>,
            Option<Vec<u8>>,
            Option<String>,
        )>(conn)
        .await
        .map_err(diesel)?;

    let same_hash = prev_key.is_some()
        && stored_hash.as_deref() == Some(image_hash.as_bytes());

    if same_hash && stored_ext.as_deref() != Some(image_ext.suffix()) {
        //
        let message = trl("error-invalid-image-extension");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %message,
            team_id = %id,
            image_version = raw_version,
            stored_extension = ?stored_ext,
            requested_extension = %image_ext.suffix(),
            operation = "reserve team avatar",
            "expected team avatar error",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        });
    }

    if same_hash {
        //
        let object_key = prev_key.ok_or_else(|| BaseError::Unrecoverable {
            message: "[reserve_avatar] pending avatar key is missing".into(),
        })?;

        return accept(TeamAvatarReservation {
            object_key,
            prev_object_key: None,
            avatar_version: u32::try_from(raw_version.ok_or_else(|| {
                BaseError::Unrecoverable {
                    message: "[reserve_avatar] avatar version is missing"
                        .into(),
                }
            })?)
            .map_err(|_| BaseError::Unrecoverable {
                message: "[reserve_avatar] avatar version is invalid".into(),
            })?,
            is_upload_required: !uploaded.unwrap_or(false),
        });
    }

    let version = next_version(raw_version.unwrap_or(0))?;

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
        is_upload_required: true,
    })
}

// Load one team info and lock the row for transactional updates.
#[instrument(level = "info", skip_all)]
async fn get_info_excluded(conn: &mut RdbConn, id: &str) -> BaseRest<TeamInfo> {
    //
    let row = t_team
        .filter(f_id.eq(id))
        .select(TeamInfoRow::as_select())
        .for_update()
        .get_result::<TeamInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let row = match row {
        //
        Some(row) => row,

        None => {
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
        }
    };

    row.try_into()
}

// Lock a team row to serialize concurrent writes in the current transaction.
#[instrument(level = "info", skip_all)]
async fn lock_team(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    let row = t_team
        .filter(f_id.eq(id))
        .select(f_id)
        .for_update()
        .get_result::<String>(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let _ = match row {
        //
        Some(row) => row,

        None => {
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
        }
    };

    accept(())
}

// Advance workset sequence and return previous value for deterministic IDs.
#[instrument(level = "info", skip_all)]
async fn increment_workset_next_index(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<i32> {
    //
    let prev = diesel::update(t_team.filter(f_id.eq(id)))
        .set(f_workset_next_index.eq(f_workset_next_index + 1))
        .returning(f_workset_next_index - 1)
        .get_result::<i32>(conn)
        .await
        .map_err(diesel)?;

    accept(prev)
}

impl<'a> Run<CreateTeam<'a>> for HybRepo {
    // Map team creation orchestration failures to the shared base error type.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Submit a team create request through repository core to keep one call path.
    async fn run(
        &self,
        oper: &CreateTeam<'_>,
    ) -> Result<TeamInfo, Self::Error> {
        submit_query!(self.core, create, oper.entry)
    }
}

impl Run<GetTeamInfo<'_>> for HybRepo {
    // Use the common base error for team info reads.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Resolve team read requests from ID-based variants and return team details.
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

impl Run<ListTeamInfos<'_>> for HybRepo {
    // Keep list query failures on a single repository-level error channel.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Return filtered and paginated team lists based on caller-provided criteria.
    async fn run(
        &self,
        oper: &ListTeamInfos<'_>,
    ) -> Result<Vec<TeamInfo>, Self::Error> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl Run<UpdateTeam<'_>> for HybRepo {
    // Keep update orchestration failures compatible with other team operations.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Route team mutation variants into the corresponding SQL update handlers.
    async fn run(&self, oper: &UpdateTeam<'_>) -> BaseRest<()> {
        match oper {
            //
            UpdateTeam::Info { repl } => {
                submit_query!(self.core, update_info, repl)
            }

            UpdateTeam::MarkAvatarUploaded { repl } => {
                submit_query!(
                    self.core,
                    mark_avatar_uploaded,
                    &repl.id,
                    repl.avatar_version,
                    repl.avatar_key.as_deref(),
                    repl.is_avatar_uploaded
                )
            }
        }
    }
}

impl Step<CreateTeam<'_>, RdbContext> for HybRepo {
    // Convert repository step failures to base error during transaction execution.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Persist a new team row within an open transaction and return persisted info.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateTeam<'_>,
    ) -> BaseRest<TeamInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl Step<UpdateTeam<'_>, RdbContext> for HybRepo {
    // Keep transactional team updates on the same base error contract.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply either profile updates or avatar flag updates in current transaction.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateTeam<'_>,
    ) -> BaseRest<()> {
        match oper {
            //
            UpdateTeam::Info { repl } => {
                update_info(context.conn(), repl).await
            }

            UpdateTeam::MarkAvatarUploaded { repl } => {
                mark_avatar_uploaded(
                    context.conn(),
                    &repl.id,
                    repl.avatar_version,
                    repl.avatar_key.as_deref(),
                    repl.is_avatar_uploaded,
                )
                .await
            }
        }
    }
}

impl Step<ReserveTeamAvatar<'_>, RdbContext> for HybRepo {
    // Report avatar-reservation validation and mutation errors through base error.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Reserve the next avatar slot and return upload reservation metadata.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ReserveTeamAvatar<'_>,
    ) -> BaseRest<TeamAvatarReservation> {
        reserve_avatar(context.conn(), oper.id, oper.image_hash, oper.image_ext)
            .await
    }
}

impl Step<GetTeamInfoExcluded<'_>, RdbContext> for HybRepo {
    // Preserve consistent error typing for locked team detail fetches.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Load team info with row lock and exclusion rules for transactional safety.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetTeamInfoExcluded<'_>,
    ) -> BaseRest<TeamInfo> {
        match oper {
            GetTeamInfoExcluded::Id { id } => {
                get_info_excluded(context.conn(), id).await
            }
        }
    }
}

impl Step<LockTeam<'_>, RdbContext> for HybRepo {
    // Keep lock contention errors on the shared repository error type.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Acquire row lock for update sequencing before sensitive team writes.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &LockTeam<'_>,
    ) -> BaseRest<()> {
        lock_team(context.conn(), oper.id).await
    }
}

impl Step<DeleteTeam<'_>, RdbContext> for HybRepo {
    // Use the common base error for hard delete operations in transactions.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Remove a team row after the caller has coordinated any dependent effects.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteTeam<'_>,
    ) -> BaseRest<()> {
        delete(context.conn(), oper.id).await
    }
}

impl Step<AllocTeamWorksetIndex<'_>, RdbContext> for HybRepo {
    // Keep index allocation failures mapped to repository base errors.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Atomically increment and return previous index for next workset reservation.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &AllocTeamWorksetIndex<'_>,
    ) -> BaseRest<i32> {
        increment_workset_next_index(context.conn(), oper.id).await
    }
}
