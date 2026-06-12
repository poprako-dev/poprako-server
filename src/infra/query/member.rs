use std::collections::HashMap;

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use time::OffsetDateTime;
use tracing::{instrument, Level};

use poprako_util::i18n::trl;
use poprako_util::page::Page;

use crate::domain::model::aggr::member::{MemberAggr, MemberForm, MemberRoleUpdate};
use crate::domain::model::aggr::team::TeamAggr;
use crate::domain::model::aggr::user::UserAggr;
use crate::domain::model::value::member_inclusion::MemberInclusion;
use crate::domain::model::value::role::RoleFlag;
use crate::domain::query::member::{MemberQuery, MemberQueryTransactional};
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::{RdbQuery, RdbQueryTransactional};
use crate::infra::query::entity::member::{MemberAspect, MemberEntry, MemberRow};
use crate::infra::query::entity::team::TeamRow;
use crate::infra::query::entity::user::UserRow;
use crate::infra::query::schema::t_member::dsl::*;
use crate::infra::query::schema::t_team::dsl as team_dsl;
use crate::infra::query::schema::t_user::dsl as user_dsl;
use crate::submit_query;

// ── Free functions ─────────────────────────────────────────────────────────

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn get_by_id(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<MemberAggr> {
    let row: MemberRow = t_member
        .filter(f_id.eq(&id))
        .select(MemberRow::as_select())
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| DomainError::expected_argument(trl("error-member-not-found")))?;

    Ok(row.into())
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn get_by_user_and_team_id(
    conn: &mut AsyncPgConnection,
    user_id: &str,
    team_id: &str,
) -> DomainResult<MemberAggr> {
    let row: MemberRow = t_member
        .filter(f_user_id.eq(user_id).and(f_team_id.eq(team_id)))
        .select(MemberRow::as_select())
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| DomainError::expected_argument(trl("error-member-not-found")))?;

    Ok(row.into())
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn list_by_team_id(
    conn: &mut AsyncPgConnection,
    team_id: &str,
    keyword: Option<&str>,
    role: Option<RoleFlag>,
    page: Page,
    includes: &MemberInclusion,
) -> DomainResult<Vec<MemberAggr>> {
    let mut query = t_member.filter(f_team_id.eq(team_id)).into_boxed();

    // Apply optional ILIKE filter on user_nickname.
    if let Some(kw) = keyword {
        let pattern = format!("%{}%", kw);
        query = query.filter(f_user_nickname.ilike(pattern));
    }

    // Apply optional role filter (IS NOT NULL on the corresponding column).
    if let Some(flag) = role {
        query = match flag {
            RoleFlag::RawProvider => query.filter(f_assigned_raw_provider_at.is_not_null()),
            RoleFlag::Translator => query.filter(f_assigned_translator_at.is_not_null()),
            RoleFlag::Proofreader => query.filter(f_assigned_proofreader_at.is_not_null()),
            RoleFlag::Typesetter => query.filter(f_assigned_typesetter_at.is_not_null()),
            RoleFlag::Redrawer => query.filter(f_assigned_redrawer_at.is_not_null()),
            RoleFlag::Reviewer => query.filter(f_assigned_reviewer_at.is_not_null()),
            RoleFlag::Publisher => query.filter(f_assigned_publisher_at.is_not_null()),
            RoleFlag::Admin => query.filter(f_assigned_admin_at.is_not_null()),
            RoleFlag::Assistant => query.filter(f_assigned_assistant_at.is_not_null()),
        };
    }

    let rows: Vec<MemberRow> = query
        .offset(page.offset as i64)
        .limit(page.limit as i64)
        .select(MemberRow::as_select())
        .load(conn)
        .await?;

    let mut result: Vec<MemberAggr> = rows.into_iter().map(|r| r.into()).collect();

    populate_inclusions(conn, &mut result, includes).await?;

    Ok(result)
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn list_by_user_id(
    conn: &mut AsyncPgConnection,
    user_id: &str,
    page: Page,
    includes: &MemberInclusion,
) -> DomainResult<Vec<MemberAggr>> {
    let rows: Vec<MemberRow> = t_member
        .filter(f_user_id.eq(user_id))
        .offset(page.offset as i64)
        .limit(page.limit as i64)
        .select(MemberRow::as_select())
        .load(conn)
        .await?;

    let mut result: Vec<MemberAggr> = rows.into_iter().map(|r| r.into()).collect();

    populate_inclusions(conn, &mut result, includes).await?;

    Ok(result)
}

async fn populate_inclusions(
    conn: &mut AsyncPgConnection,
    members: &mut [MemberAggr],
    includes: &MemberInclusion,
) -> DomainResult<()> {
    if includes.user {
        let mut user_ids: Vec<String> = members
            .iter()
            .map(|member| member.user_id.clone())
            .collect();
        user_ids.sort();
        user_ids.dedup();

        let rows: Vec<UserRow> = user_dsl::t_user
            .filter(user_dsl::f_id.eq_any(&user_ids))
            .select(UserRow::as_select())
            .load(conn)
            .await?;

        let users: HashMap<String, UserRow> = rows
            .into_iter()
            .map(|row| (row.f_id.clone(), row))
            .collect();

        for member in members.iter_mut() {
            member.user = users.get(&member.user_id).map(user_from_row);
        }
    }

    if includes.team {
        let mut team_ids: Vec<String> = members
            .iter()
            .map(|member| member.team_id.clone())
            .collect();
        team_ids.sort();
        team_ids.dedup();

        let rows: Vec<TeamRow> = team_dsl::t_team
            .filter(team_dsl::f_id.eq_any(&team_ids))
            .select(TeamRow::as_select())
            .load(conn)
            .await?;

        let teams: HashMap<String, TeamRow> = rows
            .into_iter()
            .map(|row| (row.f_id.clone(), row))
            .collect();

        for member in members.iter_mut() {
            member.team = teams.get(&member.team_id).map(team_from_row);
        }
    }

    Ok(())
}

fn user_from_row(row: &UserRow) -> UserAggr {
    UserAggr {
        id: row.f_id.clone(),
        nickname: row.f_nickname.clone(),
        qid: row.f_qid.clone(),
        avatar_key: row.f_avatar_key.clone(),
        avatar_uploaded: row.f_avatar_uploaded,
        avatar_version: row.f_avatar_version,
        is_sadmin: row.f_is_sadmin,
        last_active_at: row.f_last_active_at,
        created_at: row.f_created_at,
        updated_at: row.f_updated_at,
    }
}

fn team_from_row(row: &TeamRow) -> TeamAggr {
    TeamAggr {
        id: row.f_id.clone(),
        name: row.f_name.clone(),
        description: row.f_description.clone().unwrap_or_default(),
        avatar_key: row.f_avatar_key.clone(),
        avatar_uploaded: row.f_avatar_uploaded,
        avatar_version: row.f_avatar_version,
        workset_next_index: row.f_workset_next_index,
        created_at: row.f_created_at,
        updated_at: row.f_updated_at,
    }
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn exist_by_user_and_team_id(
    conn: &mut AsyncPgConnection,
    user_id: &str,
    team_id: &str,
) -> DomainResult<bool> {
    use diesel::dsl::{exists, select};

    let exists_result: bool = select(exists(
        t_member.filter(f_user_id.eq(user_id).and(f_team_id.eq(team_id))),
    ))
    .get_result(conn)
    .await?;

    Ok(exists_result)
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn get_by_id_excluded(
    conn: &mut AsyncPgConnection,
    id: &str,
) -> DomainResult<MemberAggr> {
    let row: MemberRow = t_member
        .filter(f_id.eq(&id))
        .select(MemberRow::as_select())
        .for_update()
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| DomainError::expected_argument(trl("error-member-not-found")))?;

    Ok(row.into())
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn get_by_user_and_team_id_excluded(
    conn: &mut AsyncPgConnection,
    user_id: &str,
    team_id: &str,
) -> DomainResult<MemberAggr> {
    let row: MemberRow = t_member
        .filter(f_user_id.eq(user_id).and(f_team_id.eq(team_id)))
        .select(MemberRow::as_select())
        .for_update()
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| DomainError::expected_argument(trl("error-member-not-found")))?;

    Ok(row.into())
}

#[instrument(err, skip(conn, form), level = Level::DEBUG)]
pub async fn create(conn: &mut AsyncPgConnection, form: &MemberForm) -> DomainResult<MemberAggr> {
    let now = OffsetDateTime::now_utc();
    let roles = form.role_mask;

    let entry = MemberEntry {
        f_id: &form.id,
        f_user_id: &form.user_id,
        f_user_nickname: &form.user_nickname,
        f_team_id: &form.team_id,
        f_assigned_raw_provider_at: roles.has_role(RoleFlag::RawProvider).then_some(now),
        f_assigned_translator_at: roles.has_role(RoleFlag::Translator).then_some(now),
        f_assigned_proofreader_at: roles.has_role(RoleFlag::Proofreader).then_some(now),
        f_assigned_typesetter_at: roles.has_role(RoleFlag::Typesetter).then_some(now),
        f_assigned_redrawer_at: roles.has_role(RoleFlag::Redrawer).then_some(now),
        f_assigned_reviewer_at: roles.has_role(RoleFlag::Reviewer).then_some(now),
        f_assigned_publisher_at: roles.has_role(RoleFlag::Publisher).then_some(now),
        f_assigned_admin_at: roles.has_role(RoleFlag::Admin).then_some(now),
        f_assigned_assistant_at: roles.has_role(RoleFlag::Assistant).then_some(now),
        f_user_last_active_at: now,
        f_created_at: now,
        f_updated_at: now,
    };

    diesel::insert_into(t_member)
        .values(&entry)
        .execute(conn)
        .await?;

    let row: MemberRow = t_member
        .filter(f_id.eq(&entry.f_id))
        .select(MemberRow::as_select())
        .first(conn)
        .await?;

    Ok(row.into())
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn update_user_nickname(
    conn: &mut AsyncPgConnection,
    user_id: &str,
    nickname: &str,
) -> DomainResult<()> {
    let now = OffsetDateTime::now_utc();

    let changes = MemberAspect::new(now).user_nickname(nickname);

    diesel::update(t_member.filter(f_user_id.eq(user_id)))
        .set(&changes)
        .execute(conn)
        .await?;

    Ok(())
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn touch_last_active(conn: &mut AsyncPgConnection, user_id: &str) -> DomainResult<()> {
    let now = OffsetDateTime::now_utc();

    let changes = MemberAspect::new(now).user_last_active_at(now);

    diesel::update(t_member.filter(f_user_id.eq(user_id)))
        .set(&changes)
        .execute(conn)
        .await?;

    Ok(())
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn list_by_user_id_excluded(
    conn: &mut AsyncPgConnection,
    user_id: &str,
) -> DomainResult<Vec<MemberAggr>> {
    let rows: Vec<MemberRow> = t_member
        .filter(f_user_id.eq(user_id))
        .for_update()
        .select(MemberRow::as_select())
        .load(conn)
        .await?;

    Ok(rows.into_iter().map(|r| r.into()).collect())
}

#[instrument(err, skip(conn, update_data), level = Level::DEBUG)]
pub async fn update_roles_fn(
    conn: &mut AsyncPgConnection,
    update_data: &MemberRoleUpdate,
) -> DomainResult<()> {
    let now = OffsetDateTime::now_utc();
    let roles = update_data.role_mask;

    // PUT-style: clear all 9 role timestamp columns, then set only those in the mask.
    let changes = MemberAspect::new(now)
        .assigned_raw_provider_at(roles.has_role(RoleFlag::RawProvider).then_some(now))
        .assigned_translator_at(roles.has_role(RoleFlag::Translator).then_some(now))
        .assigned_proofreader_at(roles.has_role(RoleFlag::Proofreader).then_some(now))
        .assigned_typesetter_at(roles.has_role(RoleFlag::Typesetter).then_some(now))
        .assigned_redrawer_at(roles.has_role(RoleFlag::Redrawer).then_some(now))
        .assigned_reviewer_at(roles.has_role(RoleFlag::Reviewer).then_some(now))
        .assigned_publisher_at(roles.has_role(RoleFlag::Publisher).then_some(now))
        .assigned_admin_at(roles.has_role(RoleFlag::Admin).then_some(now))
        .assigned_assistant_at(roles.has_role(RoleFlag::Assistant).then_some(now));

    diesel::update(t_member.filter(f_id.eq(&update_data.id)))
        .set(&changes)
        .execute(conn)
        .await?;

    Ok(())
}

#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn delete_member(conn: &mut AsyncPgConnection, member_id: &str) -> DomainResult<()> {
    diesel::delete(t_member.filter(f_id.eq(member_id)))
        .execute(conn)
        .await?;

    Ok(())
}

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait]
impl MemberQuery for RdbQuery {
    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn get_by_id(&self, id: &str) -> DomainResult<MemberAggr> {
        submit_query!(self.pool, get_by_id, id)
    }

    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn get_by_user_and_team_id(
        &self,
        user_id: &str,
        team_id: &str,
    ) -> DomainResult<MemberAggr> {
        submit_query!(self.pool, get_by_user_and_team_id, user_id, team_id)
    }

    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn list_by_team_id(
        &self,
        team_id: &str,
        keyword: Option<&str>,
        role: Option<RoleFlag>,
        page: Page,
        includes: &MemberInclusion,
    ) -> DomainResult<Vec<MemberAggr>> {
        submit_query!(
            self.pool,
            list_by_team_id,
            team_id,
            keyword,
            role,
            page,
            includes
        )
    }

    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn list_by_user_id(
        &self,
        user_id: &str,
        page: Page,
        includes: &MemberInclusion,
    ) -> DomainResult<Vec<MemberAggr>> {
        submit_query!(self.pool, list_by_user_id, user_id, page, includes)
    }

    #[instrument(err, skip(self), level = Level::DEBUG)]
    async fn exist_by_user_and_team_id(&self, user_id: &str, team_id: &str) -> DomainResult<bool> {
        submit_query!(self.pool, exist_by_user_and_team_id, user_id, team_id)
    }
}

#[async_trait]
impl<'c> MemberQueryTransactional for RdbQueryTransactional<'c> {
    async fn create(&mut self, form: &MemberForm) -> DomainResult<MemberAggr> {
        create(self.conn, form).await
    }

    async fn get_by_id_excluded(&mut self, id: &str) -> DomainResult<MemberAggr> {
        get_by_id_excluded(self.conn, id).await
    }

    async fn get_by_user_and_team_id_excluded(
        &mut self,
        user_id: &str,
        team_id: &str,
    ) -> DomainResult<MemberAggr> {
        get_by_user_and_team_id_excluded(self.conn, user_id, team_id).await
    }

    async fn update_user_nickname(&mut self, user_id: &str, nickname: &str) -> DomainResult<()> {
        update_user_nickname(self.conn, user_id, nickname).await
    }

    async fn touch_last_active(&mut self, user_id: &str) -> DomainResult<()> {
        touch_last_active(self.conn, user_id).await
    }

    async fn list_by_user_id_excluded(&mut self, user_id: &str) -> DomainResult<Vec<MemberAggr>> {
        list_by_user_id_excluded(self.conn, user_id).await
    }

    async fn update_roles(&mut self, update_data: &MemberRoleUpdate) -> DomainResult<()> {
        update_roles_fn(self.conn, update_data).await
    }

    async fn delete(&mut self, id: &str) -> DomainResult<()> {
        delete_member(self.conn, id).await
    }
}
