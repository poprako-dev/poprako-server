//! RDB-backed assignment invitation repository.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::model::assignment_invitation::{
    AssignmentInvitationForm, AssignmentInvitationInfo,
};
use crate::part::repo::assignment_invitation::{
    AssignmentInvitationRepo, AssignmentInvitationRepoTransactional,
};
use crate::part::repo::step::assignment_invitation::{
    Create, Delete, DeleteByChapterId, GetInfoByCodeExcluded, GetInfoById,
    ListInfos, MarkPendingAsUsed,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::rdb_impl::entity::assignment_invitation::{
    AssignmentInvitationAspect, AssignmentInvitationEntry,
    AssignmentInvitationRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_assignment_invitation::dsl::*;
use crate::part_impl::repo::rdb_impl::{RdbRepo, RdbRepoTransactional};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{RegularError, RegularResult};

impl AssignmentInvitationRepo<RdbContext> for RdbRepo {}

impl AssignmentInvitationRepoTransactional<RdbContext>
    for RdbRepoTransactional
{
}

/// Converts a single `AssignmentInvitationRow` into an `AssignmentInvitationInfo`.
fn row_into_info(
    row: AssignmentInvitationRow,
) -> RegularResult<AssignmentInvitationInfo> {
    row.try_into()
}

/// Converts a vector of `AssignmentInvitationRow` values into `AssignmentInvitationInfo`.
fn rows_into_infos(
    rows: Vec<AssignmentInvitationRow>,
) -> RegularResult<Vec<AssignmentInvitationInfo>> {
    rows.into_iter().map(row_into_info).collect()
}

/// Queries assignment invitation rows filtered by chapter ID and optional pending flag.
async fn list_infos(
    conn: &mut RdbConn,
    chapter_id: &str,
    pending: Option<bool>,
    offset: u64,
    limit: u64,
) -> RegularResult<Vec<AssignmentInvitationInfo>> {
    //
    let mut query = t_assignment_invitation
        .filter(f_chapter_id.eq(chapter_id))
        .into_boxed();

    if let Some(pending) = pending {
        query = query.filter(f_pending.eq(pending));
    }

    let rows: Vec<AssignmentInvitationRow> = query
        .select(AssignmentInvitationRow::as_select())
        .order_by((f_created_at.desc(), f_id.asc()))
        .offset(offset as i64)
        .limit(limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    rows_into_infos(rows)
}

/// Queries a single assignment invitation row by ID.
async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<AssignmentInvitationInfo> {
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
async fn get_info_by_code_excluded(
    conn: &mut RdbConn,
    code: &str,
) -> RegularResult<AssignmentInvitationInfo> {
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

/// Inserts a new assignment invitation row from the given form.
async fn create(
    conn: &mut RdbConn,
    form: &AssignmentInvitationForm,
) -> RegularResult<AssignmentInvitationInfo> {
    //
    let entry = AssignmentInvitationEntry::from(form);

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
async fn mark_pending_as_used(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<()> {
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

    Ok(())
}

/// Deletes a single assignment invitation row by ID.
async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    //
    diesel::delete(t_assignment_invitation.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

/// Deletes all assignment invitation rows for a given chapter ID.
async fn delete_by_chapter_id(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> RegularResult<()> {
    //
    diesel::delete(t_assignment_invitation.filter(f_chapter_id.eq(chapter_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> RegularResult<Vec<AssignmentInvitationInfo>> {
        submit_query!(
            self.core,
            list_infos,
            step.chapter_id,
            step.pending,
            step.offset,
            step.limit
        )
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> RegularResult<AssignmentInvitationInfo> {
        submit_query!(self.core, get_info_by_id, step.id)
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Create<'a>,
    ) -> RegularResult<AssignmentInvitationInfo> {
        create(context.conn(), step.form).await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoById<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoById<'a>,
    ) -> RegularResult<AssignmentInvitationInfo> {
        get_info_by_id(context.conn(), step.id).await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoByCodeExcluded<'a>, RdbContext>
    for RdbRepoTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoByCodeExcluded<'a>,
    ) -> RegularResult<AssignmentInvitationInfo> {
        get_info_by_code_excluded(context.conn(), step.code).await
    }
}

#[async_trait]
impl<'a> Advance<MarkPendingAsUsed<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &MarkPendingAsUsed<'a>,
    ) -> RegularResult<()> {
        mark_pending_as_used(context.conn(), step.id).await
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Delete<'a>,
    ) -> RegularResult<()> {
        delete(context.conn(), step.id).await
    }
}

#[async_trait]
impl<'a> Advance<DeleteByChapterId<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &DeleteByChapterId<'a>,
    ) -> RegularResult<()> {
        delete_by_chapter_id(context.conn(), step.chapter_id).await
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;
