use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use crate::domain::model::aggregate::member_invitation::MemberInvitation;
use crate::domain::query as domain_query;
use crate::domain::result::{DomainError, DomainResult};
use crate::infrastructure::query::TransactionalQuery;
use crate::infrastructure::query::entity::member_invitation::MemberInvitationRow;
use crate::infrastructure::query::schema::t_member_invitation::dsl::*;
use crate::util::err::ErrorTrace as _;
use crate::util::i18n::trl;

pub async fn get_pending_by_invitee_qid(
    conn: &mut AsyncPgConnection,
    invitee_qid: &str,
) -> DomainResult<MemberInvitation> {
    let row: MemberInvitationRow = t_member_invitation
        .filter(f_invitee_qid.eq(invitee_qid))
        .filter(f_pending.eq(true))
        .order(f_created_at.desc())
        .select(MemberInvitationRow::as_select())
        .first(conn)
        .await
        .optional()?
        .ok_or(DomainError::expected_argument(trl(
            "error-no-pending-invitation",
        )))
        .trace_debug()?;

    Ok(row.into())
}

pub async fn mark_as_used(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<()> {
    let rows_affected = diesel::update(t_member_invitation.filter(f_id.eq(id)))
        .set((
            f_pending.eq(false),
            f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(conn)
        .await?;

    if rows_affected == 0 {
        return Err(DomainError::expected_argument(trl(
            "error-invitation-not-found",
        )))
        .trace_debug();
    }

    Ok(())
}

// ── Marker traits ──────────────────────────────────────────────────────────

/// Blanket-impl marker: every [`TransactionalQuery`] is a
/// [`MemberInvitationQueryMut`](crate::domain::query::member_invitation::MemberInvitationQueryMut).
trait MemberInvitationQuery: domain_query::member_invitation::MemberInvitationQueryMut {}

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait]
impl<'c> domain_query::member_invitation::MemberInvitationQueryMut for TransactionalQuery<'c> {
    async fn get_pending_by_invitee_qid(
        &mut self,
        invitee_qid: &str,
    ) -> DomainResult<MemberInvitation> {
        get_pending_by_invitee_qid(self.conn, invitee_qid).await
    }

    async fn mark_as_used(&mut self, id: &str) -> DomainResult<()> {
        mark_as_used(self.conn, id).await
    }
}
