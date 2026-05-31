use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use crate::domain::model::aggregate::member::MemberAggr;
use crate::domain::model::aggregate::member::MemberForm;
use crate::domain::model::value::role::RoleFlag;
use crate::domain::model::value::role::RoleMask;
use crate::domain::query::member::MemberQueryTransactional;
use crate::domain::result::DomainResult;
use crate::infrastructure::query::RdbQueryTransactional;
use crate::infrastructure::query::entity::member::MemberEntry;
use crate::infrastructure::query::entity::member::MemberRow;
use crate::infrastructure::query::schema::t_member::dsl::*;

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

fn has_role(roles: &RoleMask, flag: RoleFlag) -> bool {
    let roles: u32 = (*roles).into();
    let flag: u32 = flag.into();
    roles & flag != 0
}

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait]
impl<'c> MemberQueryTransactional for RdbQueryTransactional<'c> {
    async fn create(&mut self, form: &MemberForm) -> DomainResult<MemberAggr> {
        create(self.conn, form).await
    }
}
