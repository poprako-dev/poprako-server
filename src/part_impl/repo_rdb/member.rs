//! RDB-backed member repository — free query functions and thin trait impls.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::model::member::{MemberForm, MemberInfo, MemberListSpec, MemberRoleUpdate};
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::step::member::{
    Create, Delete, FindInfoByUserIdAndTeamId, GetInfoById, GetInfoExcluded, ListInfos,
    ListInfosByUserIdExcluded, TouchLastActive, UpdateRole, UpdateUserNickname,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_rdb::entity::member::{MemberAspect, MemberEntry, MemberRow};
use crate::part_impl::repo_rdb::incl;
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional};
use crate::part_impl::shared_rdb::RdbConn;
use crate::part_impl::shared_rdb::RdbContext;
use crate::part_impl::shared_rdb::result::{diesel, expected};
use crate::result::{RegularError, RegularResult};
use crate::value::member::MemberInclOpt;
use crate::value::role::{RoleField, RoleMask};

use crate::part_impl::repo_rdb::schema::t_member::dsl::*;

impl MemberRepo<RdbContext> for RdbRepo {}

impl MemberRepoTransactional<RdbContext> for RdbRepoTransactional {}

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

fn role_timestamps_from_mask(roles: RoleMask, now: OffsetDateTime) -> RoleTimestamps {
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

fn entry_from_form<'a>(form: &'a MemberForm, now: OffsetDateTime) -> MemberEntry<'a> {
    let timestamps = role_timestamps_from_mask(form.roles, now);

    MemberEntry {
        f_id: &form.id,
        f_user_id: &form.user_id,
        f_user_nickname: &form.user_nickname,
        f_team_id: &form.team_id,
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

fn aspect_from_role_update(update: &MemberRoleUpdate, now: OffsetDateTime) -> MemberAspect<'_> {
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
    let mut escaped = String::with_capacity(input.len());

    for ch in input.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '%' => escaped.push_str("\\%"),
            '_' => escaped.push_str("\\_"),
            _ => escaped.push(ch),
        }
    }

    escaped
}

// ── Free functions ──────────────────────────────────────────────────────────

async fn find_info_by_user_id_and_team_id(
    conn: &mut RdbConn,
    user_id: &str,
    team_id: &str,
) -> RegularResult<Option<MemberInfo>> {
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

async fn list_infos(conn: &mut RdbConn, spec: &MemberListSpec) -> RegularResult<Vec<MemberInfo>> {
    let rows: Vec<MemberRow> = match spec {
        MemberListSpec::Team {
            team_id,
            fuzzy_nickname,
            role: _,
            offset,
            limit,
            ..
        } => {
            let mut query = t_member
                .filter(f_team_id.eq(team_id.as_str()))
                .select(MemberRow::as_select())
                .into_boxed();

            if let Some(nickname) = fuzzy_nickname {
                let escaped = escape_ilike_pattern(nickname);

                query = query.filter(f_user_nickname.ilike(format!("%{}%", escaped)));
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

    incl::member::populate_member_incls(conn, &mut infos, spec.incl_opt()).await?;

    Ok(infos)
}

async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[MemberInclOpt],
) -> RegularResult<MemberInfo> {
    let row: MemberRow = t_member
        .filter(f_id.eq(id))
        .select(MemberRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-member-not-found"))?;

    let mut info: MemberInfo = row.into();

    incl::member::populate_member_incls(conn, std::slice::from_mut(&mut info), incl_opt).await?;

    Ok(info)
}

async fn create(conn: &mut RdbConn, form: &MemberForm) -> RegularResult<MemberInfo> {
    let now = OffsetDateTime::now_utc();

    let entry = entry_from_form(form, now);

    let row: MemberRow = diesel::insert_into(t_member)
        .values(&entry)
        .returning(MemberRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(row.into())
}

async fn update_user_nickname(
    conn: &mut RdbConn,
    user_id: &str,
    nickname: &str,
) -> RegularResult<()> {
    let now = OffsetDateTime::now_utc();

    let aspect = MemberAspect::new(now).user_nickname(nickname);

    diesel::update(t_member.filter(f_user_id.eq(user_id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn touch_last_active(conn: &mut RdbConn, user_id: &str) -> RegularResult<()> {
    let now = OffsetDateTime::now_utc();

    let aspect = MemberAspect::new(now).user_last_active_at(now);

    diesel::update(t_member.filter(f_user_id.eq(user_id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn list_infos_by_user_id_excluded(
    conn: &mut RdbConn,
    user_id: &str,
) -> RegularResult<Vec<MemberInfo>> {
    let rows: Vec<MemberRow> = t_member
        .filter(f_user_id.eq(user_id))
        .select(MemberRow::as_select())
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

async fn get_info_excluded(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[MemberInclOpt],
) -> RegularResult<MemberInfo> {
    let row: MemberRow = t_member
        .filter(f_id.eq(id))
        .select(MemberRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-member-not-found"))?;

    let mut info: MemberInfo = row.into();

    incl::member::populate_member_incls(conn, std::slice::from_mut(&mut info), incl_opt).await?;

    Ok(info)
}

async fn update_role(conn: &mut RdbConn, update: &MemberRoleUpdate) -> RegularResult<()> {
    let now = OffsetDateTime::now_utc();

    let aspect = aspect_from_role_update(update, now);

    diesel::update(t_member.filter(f_id.eq(update.id.as_str())))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    diesel::delete(t_member.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

// ── Non-transactional: Execute impls ─────────────────────────────────────────

#[async_trait]
impl<'a> Execute<FindInfoByUserIdAndTeamId<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &FindInfoByUserIdAndTeamId<'a>,
    ) -> RegularResult<Option<MemberInfo>> {
        submit_query!(
            self.shared,
            find_info_by_user_id_and_team_id,
            step.user_id,
            step.team_id
        )
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &ListInfos<'a>) -> RegularResult<Vec<MemberInfo>> {
        submit_query!(self.shared, list_infos, step.spec)
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &GetInfoById<'a>) -> RegularResult<MemberInfo> {
        submit_query!(self.shared, get_info_by_id, step.id, step.incl_opt)
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
    ) -> RegularResult<MemberInfo> {
        create(context.conn(), step.form).await
    }
}

#[async_trait]
impl<'a> Advance<UpdateUserNickname<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &UpdateUserNickname<'a>,
    ) -> RegularResult<()> {
        update_user_nickname(context.conn(), step.user_id, step.user_nickname).await
    }
}

#[async_trait]
impl<'a> Advance<TouchLastActive<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &TouchLastActive<'a>,
    ) -> RegularResult<()> {
        touch_last_active(context.conn(), step.user_id).await
    }
}

#[async_trait]
impl<'a> Advance<ListInfosByUserIdExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ListInfosByUserIdExcluded<'a>,
    ) -> RegularResult<Vec<MemberInfo>> {
        list_infos_by_user_id_excluded(context.conn(), step.user_id).await
    }
}

#[async_trait]
impl<'a> Advance<FindInfoByUserIdAndTeamId<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &FindInfoByUserIdAndTeamId<'a>,
    ) -> RegularResult<Option<MemberInfo>> {
        find_info_by_user_id_and_team_id(context.conn(), step.user_id, step.team_id).await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoExcluded<'a>,
    ) -> RegularResult<MemberInfo> {
        get_info_excluded(context.conn(), step.id, step.incl_opt).await
    }
}

#[async_trait]
impl<'a> Advance<UpdateRole<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(&self, context: &mut RdbContext, step: &UpdateRole<'a>) -> RegularResult<()> {
        update_role(context.conn(), step.member_role_update).await
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(&self, context: &mut RdbContext, step: &Delete<'a>) -> RegularResult<()> {
        delete(context.conn(), step.id).await
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;
