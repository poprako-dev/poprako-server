//! Team lifecycle and profile persistence.

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::team::TeamComplex;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::spec::team::TeamListSpec;
use crate::model::write::team::{TeamAvatarReservation, TeamEntry, TeamRepl};
use crate::part_impl::repo::rdb_impl::entity::team::{
    TeamAspectRow, TeamEntryRow, TeamInfoRow,
};
use crate::part_impl::repo::rdb_impl::numeric::usize_from_i32;
use crate::part_impl::repo::rdb_impl::schema::t_member;
use crate::part_impl::repo::rdb_impl::schema::t_team::dsl::{
    f_avatar_extension, f_avatar_hash, f_avatar_key, f_avatar_uploaded,
    f_avatar_version, f_created_at, f_id, f_updated_at, f_workset_next_index,
    t_team,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::RdbConn;
use crate::shared::result::{diesel, next_version};
use crate::value::image::{ImageExt, ImageHash};

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

// Validate version/hash preconditions and mark avatar upload state.
/// Mark the team avatar as uploaded.
#[instrument(level = "info", skip_all)]
pub async fn mark_avatar_uploaded(
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
            //
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
            //
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
/// Reserve an avatar key and version atomically.
#[instrument(level = "info", skip_all)]
pub async fn reserve_avatar(
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
        let message = trl("error-image-extension-mismatch");

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
                //
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
