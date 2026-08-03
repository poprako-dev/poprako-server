//! RDB-backed assignment invitation repository step implementations.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::read::proj::assignment_invitation::AssignmentInvitationInfo;
use crate::model::read::spec::assignment_invitation::AssignmentInvitationListSpec;
use crate::model::write::assignment_invitation::AssignmentInvitationEntry;
use crate::part_impl::repo::rdb_impl::entity::assignment_invitation::{
    AssignmentInvitationAspectRow, AssignmentInvitationEntryRow,
    AssignmentInvitationInfoRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_assignment_invitation::dsl::*;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::RdbConn;
use crate::shared::result::diesel;

/// Queries assignment invitation rows selected by a list specification.
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    conn: &mut RdbConn,
    spec: &AssignmentInvitationListSpec,
) -> BaseRest<Vec<AssignmentInvitationInfo>> {
    //
    let mut query = t_assignment_invitation
        .filter(f_chapter_id.eq(spec.chapter_id.as_str()))
        .into_boxed();

    query = match spec.is_pending {
        //
        Some(is_pending) => query.filter(f_pending.eq(is_pending)),

        None => query,
    };

    let rows = query
        .select(AssignmentInvitationInfoRow::as_select())
        .order_by((f_created_at.desc(), f_id.asc()))
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load::<AssignmentInvitationInfoRow>(conn)
        .await
        .map_err(diesel)?;

    rows_into_infos(rows)
}

/// Queries a single assignment invitation row by ID.
#[instrument(level = "info", skip_all)]
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<AssignmentInvitationInfo> {
    //
    let row = t_assignment_invitation
        .filter(f_id.eq(id))
        .select(AssignmentInvitationInfoRow::as_select())
        .get_result::<AssignmentInvitationInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            let err_message = trl("error-invitation-not-found");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                invitation_id = %id,
                stage = "get_info_by_id",
                "expected error: assignment invitation not found",
            );

            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            }
        })?;

    row_into_info(row)
}

/// Queries a pending invitation by code under `FOR UPDATE` lock.
#[instrument(level = "info", skip_all)]
pub async fn get_info_by_code_excluded(
    conn: &mut RdbConn,
    code: &str,
) -> BaseRest<AssignmentInvitationInfo> {
    //
    let row = t_assignment_invitation
        .filter(f_code.eq(code))
        .filter(f_pending.eq(true))
        .select(AssignmentInvitationInfoRow::as_select())
        .for_update()
        .get_result::<AssignmentInvitationInfoRow>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            let err_message = trl("error-no-pending-invitation");

            tracing::warn!(
                error_variant = ?ExpectedVariant::Args,
                err_message = %err_message,
                invitation_code_length = code.len(),
                pending = true,
                stage = "get_info_by_code_excluded",
                "expected error: no pending assignment invitation",
            );

            BaseError::Expected {
                variant: ExpectedVariant::Args,
                message: err_message,
            }
        })?;

    row_into_info(row)
}

/// Inserts a new assignment invitation row from the given entry.
#[instrument(level = "info", skip_all)]
pub async fn create(
    conn: &mut RdbConn,
    model_entry: &AssignmentInvitationEntry,
) -> BaseRest<AssignmentInvitationInfo> {
    //
    let entry = AssignmentInvitationEntryRow::from(model_entry);

    let row = diesel::insert_into(t_assignment_invitation)
        .values(&entry)
        .returning(AssignmentInvitationInfoRow::as_returning())
        .get_result::<AssignmentInvitationInfoRow>(conn)
        .await
        .map_err(diesel)?;

    row_into_info(row)
}

/// Sets the pending flag to false on an invitation, marking it as used.
#[instrument(level = "info", skip_all)]
pub async fn mark_pending_as_used(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = AssignmentInvitationAspectRow::new(now).pending(false);

    let affected = diesel::update(
        t_assignment_invitation
            .filter(f_id.eq(id))
            .filter(f_pending.eq(true)),
    )
    .set(&aspect)
    .execute(conn)
    .await
    .map_err(diesel)?;

    if affected == 0 {
        //
        let err_message = trl("error-invitation-not-found");

        tracing::warn!(
            error_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            invitation_id = %id,
            pending = true,
            affected,
            stage = "mark_pending_as_used",
            "expected error: assignment invitation not found",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    accept(())
}

/// Deletes a single assignment invitation row by ID.
#[instrument(level = "info", skip_all)]
pub async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    diesel::delete(t_assignment_invitation.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Deletes an assignment invitation only while it remains pending.
#[instrument(level = "info", skip_all)]
pub async fn purge_pending(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    diesel::delete(
        t_assignment_invitation
            .filter(f_id.eq(id))
            .filter(f_pending.eq(true)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

/// Deletes all assignment invitation rows for a given chapter ID.
#[instrument(level = "info", skip_all)]
pub async fn delete_by_chapter_id(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<()> {
    //
    diesel::delete(t_assignment_invitation.filter(f_chapter_id.eq(chapter_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

// Converts a vector of `AssignmentInvitationInfoRow` values into `AssignmentInvitationInfo`.
fn rows_into_infos(
    rows: Vec<AssignmentInvitationInfoRow>,
) -> BaseRest<Vec<AssignmentInvitationInfo>> {
    rows.into_iter().map(row_into_info).collect()
}

// Converts a single `AssignmentInvitationInfoRow` into an `AssignmentInvitationInfo`.
fn row_into_info(
    row: AssignmentInvitationInfoRow,
) -> BaseRest<AssignmentInvitationInfo> {
    row.try_into()
}
