//! RDB-backed assignment invitation repository step implementations.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;
use tracing::instrument;

use crate::model::assignment_invitation::{
    AssignmentInvitationEntry, AssignmentInvitationInfo,
    AssignmentInvitationListSpec,
};
use crate::part_impl::repo::rdb_impl::entity::assignment_invitation::{
    AssignmentInvitationAspect, AssignmentInvitationRow,
    AssignmentInvitationRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_assignment_invitation::dsl::*;
use crate::part_impl::shared::RdbConn;
use crate::part_impl::shared::result::{diesel, expected};
use crate::result::{BaseRest, accept};
use crate::value::assignment_invitation::AssignmentInvitationStatus;

/// Queries assignment invitation rows selected by a list specification.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos(
    conn: &mut RdbConn,
    spec: &AssignmentInvitationListSpec,
) -> BaseRest<Vec<AssignmentInvitationInfo>> {
    //
    let mut query = t_assignment_invitation
        .filter(f_chapter_id.eq(spec.chapter_id.as_str()))
        .into_boxed();

    query = match &spec.kind {
        //
        AssignmentInvitationStatus::All => query,

        AssignmentInvitationStatus::Pending => query.filter(f_pending.eq(true)),

        AssignmentInvitationStatus::Used => query.filter(f_pending.eq(false)),
    };

    let rows: Vec<AssignmentInvitationRow> = query
        .select(AssignmentInvitationRow::as_select())
        .order_by((f_created_at.desc(), f_id.asc()))
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    rows_into_infos(rows)
}

/// Queries a single assignment invitation row by ID.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<AssignmentInvitationInfo> {
    //
    let row: AssignmentInvitationRow = t_assignment_invitation
        .filter(f_id.eq(id))
        .select(AssignmentInvitationRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-invitation-not-found"))?;

    row_into_info(row)
}

/// Queries a pending invitation by code under `FOR UPDATE` lock.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn get_info_by_code_excluded(
    conn: &mut RdbConn,
    code: &str,
) -> BaseRest<AssignmentInvitationInfo> {
    //
    let row: AssignmentInvitationRow = t_assignment_invitation
        .filter(f_code.eq(code))
        .filter(f_pending.eq(true))
        .select(AssignmentInvitationRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-no-pending-invitation"))?;

    row_into_info(row)
}

/// Inserts a new assignment invitation row from the given entry.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create(
    conn: &mut RdbConn,
    model_entry: &AssignmentInvitationEntry,
) -> BaseRest<AssignmentInvitationInfo> {
    //
    let entry = AssignmentInvitationRowEntry::from(model_entry);

    let row: AssignmentInvitationRow =
        diesel::insert_into(t_assignment_invitation)
            .values(&entry)
            .returning(AssignmentInvitationRow::as_returning())
            .get_result(conn)
            .await
            .map_err(diesel)?;

    row_into_info(row)
}

/// Sets the pending flag to false on an invitation, marking it as used.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn mark_pending_as_used(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = AssignmentInvitationAspect::new(now).pending(false);

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
        return Err(expected("error-invitation-not-found"));
    }

    accept(())
}

/// Deletes a single assignment invitation row by ID.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete(conn: &mut RdbConn, id: &str) -> BaseRest<()> {
    //
    diesel::delete(t_assignment_invitation.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Deletes an assignment invitation only while it remains pending.
#[instrument(level = "info", err(Debug), skip_all)]
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
#[instrument(level = "info", err(Debug), skip_all)]
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

// Converts a vector of `AssignmentInvitationRow` values into `AssignmentInvitationInfo`.
fn rows_into_infos(
    rows: Vec<AssignmentInvitationRow>,
) -> BaseRest<Vec<AssignmentInvitationInfo>> {
    rows.into_iter().map(row_into_info).collect()
}

// Converts a single `AssignmentInvitationRow` into an `AssignmentInvitationInfo`.
fn row_into_info(
    row: AssignmentInvitationRow,
) -> BaseRest<AssignmentInvitationInfo> {
    row.try_into()
}
