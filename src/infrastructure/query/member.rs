use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use crate::domain::model::aggregate::member::Member;
use crate::domain::model::aggregate::member::MemberForm;
use crate::domain::model::value::role::Role;
use crate::domain::model::value::role::RoleMask;
use crate::domain::query as domain_query;
use crate::domain::result::DomainRetVal;
use crate::infrastructure::query::TransactionalQuery;
use crate::infrastructure::query::entity::member::MemberEntry;
use crate::infrastructure::query::entity::member::MemberRow;
use crate::infrastructure::query::schema::t_member::dsl::*;

pub async fn create(conn: &mut AsyncPgConnection, form: &MemberForm) -> DomainRetVal<Member> {
    let now = OffsetDateTime::now_utc();
    let roles = form.roles;

    let entry = MemberEntry {
        f_id: form.id.clone(),
        f_user_id: form.user_id.clone(),
        f_user_nickname: form.user_nickname.clone(),
        f_team_id: form.team_id.clone(),
        f_assigned_raw_provider_at: role_bit(&roles, Role::RawProvider.into()).then_some(now),
        f_assigned_translator_at: role_bit(&roles, Role::Translator.into()).then_some(now),
        f_assigned_proofreader_at: role_bit(&roles, Role::Proofreader.into()).then_some(now),
        f_assigned_typesetter_at: role_bit(&roles, Role::Typesetter.into()).then_some(now),
        f_assigned_redrawer_at: role_bit(&roles, Role::Redrawer.into()).then_some(now),
        f_assigned_reviewer_at: role_bit(&roles, Role::Reviewer.into()).then_some(now),
        f_assigned_publisher_at: role_bit(&roles, Role::Publisher.into()).then_some(now),
        f_assigned_admin_at: role_bit(&roles, Role::Admin.into()).then_some(now),
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

trait MemberQuery: domain_query::member::MemberQueryMut {}

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl<'c> domain_query::member::MemberQueryMut for TransactionalQuery<'c> {
    async fn create(&mut self, form: MemberForm) -> DomainRetVal<Member> {
        create(self.conn, &form).await
    }
}
