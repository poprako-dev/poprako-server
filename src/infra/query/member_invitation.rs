use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use tracing::Level;
use tracing::instrument;

use crate::domain::model::aggr::member_invitation::MemberInvitationAggr;
use crate::domain::query::member_invitation::MemberInvitationQueryTransactional;
use crate::domain::result::{DomainError, DomainResult};
use crate::infra::query::RdbQueryTransactional;
use crate::infra::query::entity::member_invitation::MemberInvitationAspect;
use crate::infra::query::entity::member_invitation::MemberInvitationRow;
use crate::infra::query::schema::t_member_invitation::dsl::*;
use poprako_util::i18n::trl;

/// SELECT ... FOR UPDATE: returns the pending invitation for the given code
/// with an exclusive row lock, or an expected error if none matches.
#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn get_by_code_ex(
    conn: &mut AsyncPgConnection,
    invitation_code: &str,
) -> DomainResult<MemberInvitationAggr> {
    let row: MemberInvitationRow = t_member_invitation
        .filter(f_invitation_code.eq(&invitation_code))
        .filter(f_pending.eq(true))
        .select(MemberInvitationRow::as_select())
        .for_update()
        .first(conn)
        .await
        .optional()?
        .ok_or_else(|| DomainError::expected_argument(trl("error-no-pending-invitation")))?;

    Ok(row.into())
}

/// Conditionally marks an invitation as consumed.
///
/// The `WHERE f_pending = true` guard ensures this is a no-op on an already-consumed row,
/// which acts as a safety net regardless of the row lock held by [`get_by_code_ex`].
#[instrument(err, skip(conn), level = Level::DEBUG)]
pub async fn mark_pending_as_used(conn: &mut AsyncPgConnection, id: &str) -> DomainResult<()> {
    let now = OffsetDateTime::now_utc();

    let aspect = MemberInvitationAspect {
        f_pending: false,
        f_updated_at: now,
    };

    let rows_affected = diesel::update(
        t_member_invitation
            .filter(f_id.eq(id))
            .filter(f_pending.eq(true)),
    )
    .set(&aspect)
    .execute(conn)
    .await?;

    if rows_affected == 0 {
        return Err(DomainError::expected_argument(trl(
            "error-invitation-not-found",
        )));
    }

    Ok(())
}

// ── impls ──────────────────────────────────────────────────────────────────

#[async_trait]
impl<'c> MemberInvitationQueryTransactional for RdbQueryTransactional<'c> {
    async fn get_by_code_ex(
        &mut self,
        invitation_code: &str,
    ) -> DomainResult<MemberInvitationAggr> {
        get_by_code_ex(self.conn, invitation_code).await
    }

    async fn mark_pending_as_used(&mut self, id: &str) -> DomainResult<()> {
        mark_pending_as_used(self.conn, id).await
    }
}
