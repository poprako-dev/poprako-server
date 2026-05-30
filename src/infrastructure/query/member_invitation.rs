use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use crate::domain::model::aggregate::member_invitation::MemberInvitation;
use crate::domain::query::member_invitation::MemberInvitationQueryTransactional;
use crate::domain::result::{DomainError, DomainResult};
use crate::infrastructure::query::QueryTransactional;
use crate::infrastructure::query::entity::member_invitation::MemberInvitationRow;
use crate::infrastructure::query::schema::t_member_invitation::dsl::*;
use crate::util::err::ErrorTrace as _;
use crate::util::i18n::trl;

/// SELECT ... FOR UPDATE: returns the pending invitation for the given code
/// with an exclusive row lock, or an expected error if none matches.
pub async fn get_by_code_ex(
    conn: &mut AsyncPgConnection,
    invitation_code: String,
) -> DomainResult<MemberInvitation> {
    let row: MemberInvitationRow = t_member_invitation
        .filter(f_invitation_code.eq(&invitation_code))
        .filter(f_pending.eq(true))
        .select(MemberInvitationRow::as_select())
        .for_update()
        .first(conn)
        .await
        .optional()?
        .ok_or(DomainError::expected_argument(trl(
            "error-no-pending-invitation",
        )))
        .trace_debug()?;

    Ok(row.into())
}

/// Conditionally marks an invitation as consumed.
///
/// The `WHERE f_pending = true` guard ensures this is a no-op on an already-consumed row,
/// which acts as a safety net regardless of the row lock held by [`get_by_code_ex`].
pub async fn mark_pending_as_used(conn: &mut AsyncPgConnection, id: String) -> DomainResult<()> {
    let rows_affected = diesel::update(
        t_member_invitation
            .filter(f_id.eq(id))
            .filter(f_pending.eq(true)),
    )
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

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait]
impl<'c> MemberInvitationQueryTransactional for QueryTransactional<'c> {
    async fn get_by_code_ex(&mut self, invitation_code: String) -> DomainResult<MemberInvitation> {
        get_by_code_ex(self.conn, invitation_code).await
    }

    async fn mark_pending_as_used(&mut self, id: String) -> DomainResult<()> {
        mark_pending_as_used(self.conn, id).await
    }
}
