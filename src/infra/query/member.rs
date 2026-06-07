use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use tracing::Level;
use tracing::instrument;

use crate::domain::model::aggr::member::MemberAggr;
use crate::domain::model::aggr::member::MemberForm;
use crate::domain::model::value::role::RoleFlag;
use crate::domain::query::member::MemberQueryTransactional;
use crate::domain::result::DomainResult;
use crate::infra::query::RdbQueryTransactional;
use crate::infra::query::entity::member::MemberAspect;
use crate::infra::query::entity::member::MemberEntry;
use crate::infra::query::entity::member::MemberRow;
use crate::infra::query::schema::t_member::dsl::*;

#[instrument(err, skip(conn, form), level = Level::DEBUG)]
pub async fn create(conn: &mut AsyncPgConnection, form: &MemberForm) -> DomainResult<MemberAggr> {
    let now = OffsetDateTime::now_utc();
    let roles = form.roles;

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

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait]
impl<'c> MemberQueryTransactional for RdbQueryTransactional<'c> {
    async fn create(&mut self, form: &MemberForm) -> DomainResult<MemberAggr> {
        create(self.conn, form).await
    }

    async fn update_user_nickname(&mut self, user_id: &str, nickname: &str) -> DomainResult<()> {
        update_user_nickname(self.conn, user_id, nickname).await
    }

    async fn touch_last_active(&mut self, user_id: &str) -> DomainResult<()> {
        touch_last_active(self.conn, user_id).await
    }
}
