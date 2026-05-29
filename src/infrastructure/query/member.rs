use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use crate::domain::model::aggregate::member::Member;
use crate::domain::model::aggregate::member::MemberForm;
use crate::domain::model::value::role::RoleFlag;
use crate::domain::model::value::role::RoleMask;
use crate::domain::query as domain_query;
use crate::domain::result::DomainResult;
use crate::infrastructure::query::TransactionalQuery;
use crate::infrastructure::query::entity::member::MemberEntry;
use crate::infrastructure::query::entity::member::MemberRow;
use crate::infrastructure::query::schema::t_member::dsl::*;

pub async fn create(conn: &mut AsyncPgConnection, form: &MemberForm) -> DomainResult<Member> {
    let now = OffsetDateTime::now_utc();
    let roles = form.roles;

    let entry = MemberEntry {
        f_id: form.id.clone(),
        f_user_id: form.user_id.clone(),
        f_user_nickname: form.user_nickname.clone(),
        f_team_id: form.team_id.clone(),
        f_assigned_raw_provider_at: role_bit(&roles, RoleFlag::RawProvider.into()).then_some(now),
        f_assigned_translator_at: role_bit(&roles, RoleFlag::Translator.into()).then_some(now),
        f_assigned_proofreader_at: role_bit(&roles, RoleFlag::Proofreader.into()).then_some(now),
        f_assigned_typesetter_at: role_bit(&roles, RoleFlag::Typesetter.into()).then_some(now),
        f_assigned_redrawer_at: role_bit(&roles, RoleFlag::Redrawer.into()).then_some(now),
        f_assigned_reviewer_at: role_bit(&roles, RoleFlag::Reviewer.into()).then_some(now),
        f_assigned_publisher_at: role_bit(&roles, RoleFlag::Publisher.into()).then_some(now),
        f_assigned_admin_at: role_bit(&roles, RoleFlag::Admin.into()).then_some(now),
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

fn role_bit(roles: &RoleMask, role_val: u32) -> bool {
    let inner: u32 = (*roles).into();
    inner & role_val != 0
}

// ── Marker traits ──────────────────────────────────────────────────────────

/// Blanket-impl marker: every [`TransactionalQuery`] is a
/// [`MemberQueryMut`](crate::domain::query::member::MemberQueryMut).
trait MemberQuery: domain_query::member::MemberQueryMut {}

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait]
impl<'c> domain_query::member::MemberQueryMut for TransactionalQuery<'c> {
    async fn create(&mut self, form: MemberForm) -> DomainResult<Member> {
        create(self.conn, &form).await
    }
}
