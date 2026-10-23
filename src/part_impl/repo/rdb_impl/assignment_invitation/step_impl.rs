//! RDB-backed assignment invitation repository step implementations.

use diesel::prelude::{
    ExpressionMethods as _, OptionalExtension as _, QueryDsl as _,
    SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use time::OffsetDateTime;
use tracing::instrument;

use poprako_rdb_core::RdbConn;
use poprako_util::i18n::trl;

use crate::model::read::proj::assignment_invitation::AssignmentInvitationInfo;
use crate::model::read::spec::assignment_invitation::AssignmentInvitationListSpec;
use crate::model::write::assignment_invitation::AssignmentInvitationEntry;
use crate::part_impl::repo::rdb_impl::entity::assignment_invitation::{
    AssignmentInvitationInfoRecord, AssignmentInvitationItem,
    AssignmentInvitationPack,
};
use crate::part_impl::repo::rdb_impl::schema::t_assignment_invitation::dsl::{
    f_chapter_id, f_code, f_created_at, f_id, f_pending,
    t_assignment_invitation,
};
use crate::result::{BaseRest, ExpectedVariant, accept};
use crate::shared::result::diesel;

/// Queries assignment invitation records selected by a list specification.
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

    let records = query
        .select(AssignmentInvitationInfoRecord::as_select())
        .order_by((f_created_at.desc(), f_id.asc()))
        .offset(i64::from(spec.offset))
        .limit(i64::from(spec.limit.get()))
        .load::<AssignmentInvitationInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    records_into_infos(records)
}

/// Queries a single assignment invitation record by ID.
#[instrument(level = "info", skip_all)]
pub async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<AssignmentInvitationInfo> {
    //
    let record = t_assignment_invitation
        .filter(f_id.eq(id))
        .select(AssignmentInvitationInfoRecord::as_select())
        .get_result::<AssignmentInvitationInfoRecord>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            expected_error! {
                ExpectedVariant::Args,
                message: trl("error-invitation-not-found");
                error_variant = ?ExpectedVariant::Args,
                invitation_id = %id,
                stage = "get_info_by_id",
                "assignment invitation not found",
            }
        })?;

    record_into_info(record)
}

/// Queries a pending invitation by code under `FOR UPDATE` lock.
#[instrument(level = "info", skip_all)]
pub async fn get_info_by_code_excluded(
    conn: &mut RdbConn,
    code: &str,
) -> BaseRest<AssignmentInvitationInfo> {
    //
    let record = t_assignment_invitation
        .filter(f_code.eq(code))
        .filter(f_pending.eq(true))
        .select(AssignmentInvitationInfoRecord::as_select())
        .for_update()
        .get_result::<AssignmentInvitationInfoRecord>(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| {
            //
            expected_error! {
                ExpectedVariant::Args,
                message: trl("error-no-pending-invitation");
                error_variant = ?ExpectedVariant::Args,
                invitation_code_length = code.len(),
                pending = true,
                stage = "get_info_by_code_excluded",
                "no pending assignment invitation",
            }
        })?;

    record_into_info(record)
}

/// Inserts a new assignment invitation record from the given entry.
#[instrument(level = "info", skip_all)]
pub async fn create(
    conn: &mut RdbConn,
    entry: &AssignmentInvitationEntry,
) -> BaseRest<AssignmentInvitationInfo> {
    //
    let item = AssignmentInvitationItem::from(entry);

    let record = diesel::insert_into(t_assignment_invitation)
        .values(&item)
        .returning(AssignmentInvitationInfoRecord::as_returning())
        .get_result::<AssignmentInvitationInfoRecord>(conn)
        .await
        .map_err(diesel)?;

    record_into_info(record)
}

/// Sets the pending flag to false on an invitation, marking it as used.
#[instrument(level = "info", skip_all)]
pub async fn mark_pending_as_used(
    conn: &mut RdbConn,
    id: &str,
) -> BaseRest<()> {
    //
    let now = OffsetDateTime::now_utc();

    let pack = AssignmentInvitationPack::new(now).pending(false);

    let affected = diesel::update(
        t_assignment_invitation
            .filter(f_id.eq(id))
            .filter(f_pending.eq(true)),
    )
    .set(&pack)
    .execute(conn)
    .await
    .map_err(diesel)?;

    if affected == 0 {
        //
        return Err(expected_error! {
            ExpectedVariant::Args,
            message: trl("error-invitation-not-found");
            error_variant = ?ExpectedVariant::Args,
            invitation_id = %id,
            pending = true,
            affected,
            stage = "mark_pending_as_used",
            "assignment invitation not found",
        });
    }

    accept(())
}

/// Deletes a single assignment invitation record by ID.
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

/// Deletes all assignment invitation records for a given chapter ID.
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

// Converts a vector of `AssignmentInvitationInfoRecord` values into `AssignmentInvitationInfo`.
fn records_into_infos(
    records: Vec<AssignmentInvitationInfoRecord>,
) -> BaseRest<Vec<AssignmentInvitationInfo>> {
    records.into_iter().map(record_into_info).collect()
}

// Converts a single `AssignmentInvitationInfoRecord` into an `AssignmentInvitationInfo`.
fn record_into_info(
    record: AssignmentInvitationInfoRecord,
) -> BaseRest<AssignmentInvitationInfo> {
    record.try_into()
}
