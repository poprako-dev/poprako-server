//! RDB-backed member repository — free-query helper functions.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::read::proj::member::MemberInfo;
use crate::model::read::spec::member::MemberListSpec;
use crate::model::write::member::{MemberEntry, MemberRoleRepl};
use crate::part_impl::repo::rdb_impl::entity::member::{
    MemberAspect, MemberRow, MemberRowEntry,
};
use crate::part_impl::repo::rdb_impl::incl;
use crate::part_impl::repo::rdb_impl::schema::t_member::dsl::*;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::RdbConn;
use crate::shared::result::diesel;
use crate::value::member::MemberInclOpt;
use crate::value::role::{RoleField, RoleMask};

// ── Free functions ──────────────────────────────────────────────────────────

/// Look up a member by user and team IDs.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn find_info_by_user_id_and_team_id(
    conn: &mut RdbConn,
    user_id: &str,
    team_id: &str,
) -> BaseRest<Option<MemberInfo>> {
    //
    let row: Option<MemberRow> = t_member
        .filter(f_user_id.eq(user_id))
        .filter(f_team_id.eq(team_id))
        .select(MemberRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    accept(row.map(Into::into))
}

/// Query a paginated, filtered list of member infos.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos(
    conn: &mut RdbConn,
    spec: &MemberListSpec,
) -> BaseRest<Vec<MemberInfo>> {
    //
    let rows: Vec<MemberRow> =
        match spec {
            //
            MemberListSpec::Team {
                team_id,
                fuzzy_nickname,
                role,
                offset,
                limit,
                ..
            } => {
                //
                let mut query = t_member
                    .filter(f_team_id.eq(team_id.as_str()))
                    .select(MemberRow::as_select())
                    .into_boxed();

                if let Some(nickname) = fuzzy_nickname {
                    //
                    let escaped = escape_ilike_pattern(nickname);

                    query = query.filter(
                        f_user_nickname.ilike(format!("%{}%", escaped)),
                    );
                }

                if let Some(role) = role {
                    query =
                        match *role {
                            //
                            RoleField::RAW_PROVIDER => query.filter(
                                f_assigned_raw_provider_at.is_not_null(),
                            ),

                            RoleField::TRANSLATOR => query
                                .filter(f_assigned_translator_at.is_not_null()),

                            RoleField::PROOFREADER => query.filter(
                                f_assigned_proofreader_at.is_not_null(),
                            ),

                            RoleField::TYPESETTER => query
                                .filter(f_assigned_typesetter_at.is_not_null()),

                            RoleField::REDRAWER => query
                                .filter(f_assigned_redrawer_at.is_not_null()),

                            RoleField::REVIEWER => query
                                .filter(f_assigned_reviewer_at.is_not_null()),

                            RoleField::PUBLISHER => query
                                .filter(f_assigned_publisher_at.is_not_null()),

                            RoleField::ADMIN => {
                                query.filter(f_assigned_admin_at.is_not_null())
                            }

                            RoleField::BOT => {
                                query.filter(f_assigned_bot_at.is_not_null())
                            }

                            _ => query,
                        };
                }

                query
                    .order_by(f_user_last_active_at.desc())
                    .offset((*offset) as i64)
                    .limit((*limit) as i64)
                    .load(conn)
                    .await
                    .map_err(diesel)?
            }

            MemberListSpec::User {
                owner_id,
                offset,
                limit,
                ..
            } => t_member
                .filter(f_user_id.eq(owner_id.as_str()))
                .select(MemberRow::as_select())
                .order_by(f_user_last_active_at.desc())
                .offset((*offset) as i64)
                .limit((*limit) as i64)
                .load(conn)
                .await
                .map_err(diesel)?,
        };

    let mut infos = rows
        .into_iter()
        .map(Into::into)
        .collect::<Vec<MemberInfo>>();

    incl::member::populate_member_incls(conn, &mut infos, spec.incl_opt())
        .await?;

    accept(infos)
}

/// Load a single member info by ID with optional includes.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[MemberInclOpt],
) -> BaseRest<MemberInfo> {
    //
    let row: Option<MemberRow> = t_member
        .filter(f_id.eq(id))
        .select(MemberRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let row = match row {
        //
        Some(row) => row,

        None => {
            //
            let message = trl("error-member-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %message,
                member_id = %id,
                operation = "get member info",
                "expected member error",
            );

            return Err(BaseError::Expected {
                variant: ExpectedVariant::Args,
                message,
            });
        }
    };

    let mut info: MemberInfo = row.into();

    incl::member::populate_member_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    accept(info)
}

/// Insert a new member and return its info.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create(
    conn: &mut RdbConn,
    entry: &MemberEntry,
) -> BaseRest<MemberInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let entry = entity_from_entry(entry, now);

    let row: MemberRow = diesel::insert_into(t_member)
        .values(&entry)
        .returning(MemberRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    accept(row.into())
}

/// Update the user-nickname for every member row owned by the given user.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn update_user_nickname(
    conn: &mut RdbConn,
    user_id: &str,
    nickname: &str,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = MemberAspect::new(now).user_nickname(nickname);

    diesel::update(t_member.filter(f_user_id.eq(user_id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Query all member infos for a user, locking the rows for update.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos_by_user_id_excluded(
    conn: &mut RdbConn,
    user_id: &str,
) -> BaseRest<Vec<MemberInfo>> {
    //
    let rows: Vec<MemberRow> = t_member
        .filter(f_user_id.eq(user_id))
        .select(MemberRow::as_select())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

/// Query all member infos for a team, locking the rows for update.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos_by_team_id_excluded(
    conn: &mut RdbConn,
    team_id: &str,
) -> BaseRest<Vec<MemberInfo>> {
    //
    let rows: Vec<MemberRow> = t_member
        .filter(f_team_id.eq(team_id))
        .select(MemberRow::as_select())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

/// Query all member infos for a user without acquiring a lock.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos_by_user_id(
    conn: &mut RdbConn,
    user_id: &str,
) -> BaseRest<Vec<MemberInfo>> {
    //
    let rows: Vec<MemberRow> = t_member
        .filter(f_user_id.eq(user_id))
        .select(MemberRow::as_select())
        .load(conn)
        .await
        .map_err(diesel)?;

    accept(rows.into_iter().map(Into::into).collect())
}

/// Update the role mask and refresh assignment timestamps for a member.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn update_role(
    conn: &mut RdbConn,
    update: &MemberRoleRepl,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = aspect_from_role_update(update, now);

    diesel::update(t_member.filter(f_id.eq(update.id.as_str())))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Delete a member by ID.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    diesel::delete(t_member.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Escape SQL ILIKE wildcards so user input remains literal.
fn escape_ilike_pattern(input: &str) -> String {
    //
    let mut escaped = String::with_capacity(input.len());

    for ch in input.chars() {
        match ch {
            //
            '\\' => escaped.push_str("\\\\"),

            '%' => escaped.push_str("\\%"),

            '_' => escaped.push_str("\\_"),

            _ => escaped.push(ch),
        }
    }

    escaped
}

// Track per-role assignment timestamps captured from a role mask.
struct RoleTimestamps {
    //
    // Timestamp of raw provider assignment; absent means role not enabled.
    raw_provider: Option<OffsetDateTime>,

    // Timestamp of translator assignment; absent means role not enabled.
    translator: Option<OffsetDateTime>,

    // Timestamp of proofreader assignment; absent means role not enabled.
    proofreader: Option<OffsetDateTime>,

    // Timestamp of typesetter assignment; absent means role not enabled.
    typesetter: Option<OffsetDateTime>,

    // Timestamp of redrawer assignment; absent means role not enabled.
    redrawer: Option<OffsetDateTime>,

    // Timestamp of reviewer assignment; absent means role not enabled.
    reviewer: Option<OffsetDateTime>,

    // Timestamp of publisher assignment; absent means role not enabled.
    publisher: Option<OffsetDateTime>,

    // Timestamp of admin assignment; absent means role not enabled.
    admin: Option<OffsetDateTime>,

    // Timestamp of bot assignment; absent means role not enabled.
    bot: Option<OffsetDateTime>,
}

// Convert MemberEntry into insert payload, including per-role timestamp defaults.
fn entity_from_entry<'a>(
    entry: &'a MemberEntry,
    now: OffsetDateTime,
) -> MemberRowEntry<'a> {
    //
    let timestamps = role_timestamps_from_mask(entry.roles, now);

    MemberRowEntry {
        f_id: &entry.id,
        f_user_id: &entry.user_id,
        f_user_nickname: &entry.user_nickname,
        f_team_id: &entry.team_id,
        f_assigned_raw_provider_at: timestamps.raw_provider,
        f_assigned_translator_at: timestamps.translator,
        f_assigned_proofreader_at: timestamps.proofreader,
        f_assigned_typesetter_at: timestamps.typesetter,
        f_assigned_redrawer_at: timestamps.redrawer,
        f_assigned_reviewer_at: timestamps.reviewer,
        f_assigned_publisher_at: timestamps.publisher,
        f_assigned_admin_at: timestamps.admin,
        f_assigned_bot_at: timestamps.bot,
        f_user_last_active_at: now,
        f_created_at: now,
        f_updated_at: now,
    }
}

// Build update aspect from role change payload, applying now-stamped enabled roles.
fn aspect_from_role_update(
    update: &MemberRoleRepl,
    now: OffsetDateTime,
) -> MemberAspect<'_> {
    //
    let timestamps = role_timestamps_from_mask(update.roles, now);

    let mut aspect = MemberAspect::new(now);

    aspect = aspect
        .assigned_raw_provider_at(timestamps.raw_provider)
        .assigned_translator_at(timestamps.translator)
        .assigned_proofreader_at(timestamps.proofreader)
        .assigned_typesetter_at(timestamps.typesetter)
        .assigned_redrawer_at(timestamps.redrawer)
        .assigned_reviewer_at(timestamps.reviewer)
        .assigned_publisher_at(timestamps.publisher)
        .assigned_admin_at(timestamps.admin)
        .assigned_bot_at(timestamps.bot);

    aspect
}

// Convert role mask into assignment timestamps by stamping enabled roles with now.
fn role_timestamps_from_mask(
    roles: RoleMask,
    now: OffsetDateTime,
) -> RoleTimestamps {
    //
    let timestamp_fn = |field: RoleField| -> Option<OffsetDateTime> {
        roles.has_any_role(&[field]).then_some(now)
    };

    RoleTimestamps {
        raw_provider: timestamp_fn(RoleField::RAW_PROVIDER),
        translator: timestamp_fn(RoleField::TRANSLATOR),
        proofreader: timestamp_fn(RoleField::PROOFREADER),
        typesetter: timestamp_fn(RoleField::TYPESETTER),
        redrawer: timestamp_fn(RoleField::REDRAWER),
        reviewer: timestamp_fn(RoleField::REVIEWER),
        publisher: timestamp_fn(RoleField::PUBLISHER),
        admin: timestamp_fn(RoleField::ADMIN),
        bot: None,
    }
}
