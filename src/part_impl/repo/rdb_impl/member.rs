//! RDB-backed member repository — free query functions and thin trait impls.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_orchestra::Run;

use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part_impl::repo::rdb_impl::entity::member::{
    MemberAspect, MemberRow, MemberRowEntry,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, incl};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{RegularError, RegularResult};
use crate::value::member::MemberInclOpt;
use crate::value::role::{RoleField, RoleMask};

use crate::model::member::{
    MemberEntry, MemberInfo, MemberListSpec, MemberRoleUpdate,
};
use crate::part_impl::repo::rdb_impl::schema::t_member::dsl::*;

impl MemberRepo<RdbContext> for RdbRepo {}

mod orchestra;

/// Per-role assignment timestamps extracted from a [`RoleMask`].
struct RoleTimestamps {
    raw_provider: Option<OffsetDateTime>,
    translator: Option<OffsetDateTime>,
    proofreader: Option<OffsetDateTime>,
    typesetter: Option<OffsetDateTime>,
    redrawer: Option<OffsetDateTime>,
    reviewer: Option<OffsetDateTime>,
    publisher: Option<OffsetDateTime>,
    admin: Option<OffsetDateTime>,
    bot: Option<OffsetDateTime>,
}

/// Compute a [`RoleTimestamps`] from a [`RoleMask`], setting each role's
/// timestamp to `now` when that role is present in the mask.
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

/// Build a [`MemberRowEntry`] for insertion from a [`MemberEntry`] and the
/// current timestamp.
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

/// Build a [`MemberAspect`] from a [`MemberRoleUpdate`], stamping each
/// assigned role's timestamp to `now`.
fn aspect_from_role_update(
    update: &MemberRoleUpdate,
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

/// Escape PostgreSQL `ILIKE` wildcard characters in a user-supplied search term.
///
/// The characters `%`, `_`, and `\` have special meaning in `LIKE`/`ILIKE`
/// patterns and must be escaped to prevent accidental (or malicious) wildcard
/// injection when the term is embedded in a pattern like `"%{}%"`.
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

// ── Free functions ──────────────────────────────────────────────────────────

/// Look up a member by user and team IDs.
async fn find_info_by_user_id_and_team_id(
    conn: &mut RdbConn,
    user_id: &str,
    team_id: &str,
) -> RegularResult<Option<MemberInfo>> {
    //
    let row: Option<MemberRow> = t_member
        .filter(f_user_id.eq(user_id))
        .filter(f_team_id.eq(team_id))
        .select(MemberRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    Ok(row.map(Into::into))
}

impl<'a> Run<FindMemberInfo<'a>> for RdbRepo {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &FindMemberInfo<'a>,
    ) -> RegularResult<Option<MemberInfo>> {
        match oper {
            FindMemberInfo::UserTeam { user_id, team_id } => {
                submit_query!(
                    self.core,
                    find_info_by_user_id_and_team_id,
                    user_id,
                    team_id
                )
            }
        }
    }
}

/// Query a paginated, filtered list of member infos.
async fn list_infos(
    conn: &mut RdbConn,
    spec: &MemberListSpec,
) -> RegularResult<Vec<MemberInfo>> {
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

    let mut infos: Vec<MemberInfo> = rows.into_iter().map(Into::into).collect();

    incl::member::populate_member_incls(conn, &mut infos, spec.incl_opt())
        .await?;

    Ok(infos)
}

/// Load a single member info by ID with optional includes.
async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[MemberInclOpt],
) -> RegularResult<MemberInfo> {
    //
    let row: MemberRow = t_member
        .filter(f_id.eq(id))
        .select(MemberRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-member-not-found"))?;

    let mut info: MemberInfo = row.into();

    incl::member::populate_member_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    Ok(info)
}

/// Insert a new member and return its info.
async fn create(
    conn: &mut RdbConn,
    entry: &MemberEntry,
) -> RegularResult<MemberInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let entry = entity_from_entry(entry, now);

    let row: MemberRow = diesel::insert_into(t_member)
        .values(&entry)
        .returning(MemberRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(row.into())
}

/// Update the user-nickname for every member row owned by the given user.
async fn update_user_nickname(
    conn: &mut RdbConn,
    user_id: &str,
    nickname: &str,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = MemberAspect::new(now).user_nickname(nickname);

    diesel::update(t_member.filter(f_user_id.eq(user_id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Query all member infos for a user, locking the rows for update.
async fn list_infos_by_user_id_excluded(
    conn: &mut RdbConn,
    user_id: &str,
) -> RegularResult<Vec<MemberInfo>> {
    //
    let rows: Vec<MemberRow> = t_member
        .filter(f_user_id.eq(user_id))
        .select(MemberRow::as_select())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Query all member infos for a user without acquiring a lock.
async fn list_infos_by_user_id(
    conn: &mut RdbConn,
    user_id: &str,
) -> RegularResult<Vec<MemberInfo>> {
    //
    let rows: Vec<MemberRow> = t_member
        .filter(f_user_id.eq(user_id))
        .select(MemberRow::as_select())
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

/// Update the role mask and refresh assignment timestamps for a member.
async fn update_role(
    conn: &mut RdbConn,
    update: &MemberRoleUpdate,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = aspect_from_role_update(update, now);

    diesel::update(t_member.filter(f_id.eq(update.id.as_str())))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Delete a member by ID.
async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    //
    diesel::delete(t_member.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

#[cfg(all(test, feature = "repo"))]
mod tests;
