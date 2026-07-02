//! RDB-backed assignment invitation repository.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::model::assignment_invitation::{AssignmentInvitationForm, AssignmentInvitationInfo};
use crate::part::repo::assignment_invitation::{
    AssignmentInvitationRepo, AssignmentInvitationRepoTransactional,
};
use crate::part::repo::step::assignment_invitation::{
    Create, Delete, GetInfoByCodeExcluded, GetInfoById, ListInfos, MarkPendingAsUsed,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_rdb::entity::assignment_invitation::{
    AssignmentInvitationAspect, AssignmentInvitationEntry, AssignmentInvitationRow,
};
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional};
use crate::part_impl::repo_rdb::dsl;
use crate::part_impl::shared_rdb::RdbConn;
use crate::part_impl::shared_rdb::RdbContext;
use crate::part_impl::shared_rdb::result::{diesel, expected};
use crate::result::{RegularError, RegularResult};

// NOTE: use dsl::* is the Diesel impl layer exception to rust-use-style
use dsl::*;
use dsl::t_assignment_invitation::*;

impl AssignmentInvitationRepo<RdbContext> for RdbRepo {}

impl AssignmentInvitationRepoTransactional<RdbContext> for RdbRepoTransactional {}

fn row_into_info(row: AssignmentInvitationRow) -> RegularResult<AssignmentInvitationInfo> {
    row.try_into()
}

fn rows_into_infos(
    rows: Vec<AssignmentInvitationRow>,
) -> RegularResult<Vec<AssignmentInvitationInfo>> {
    rows.into_iter().map(row_into_info).collect()
}

async fn list_infos(
    conn: &mut RdbConn,
    chapter_id: &str,
    pending: Option<bool>,
    offset: u64,
    limit: u64,
) -> RegularResult<Vec<AssignmentInvitationInfo>> {
    let mut query = t_assignment_invitation
        .filter(f_chapter_id.eq(chapter_id))
        .into_boxed();

    if let Some(pending) = pending {
        query = query.filter(f_pending.eq(pending));
    }

    let rows: Vec<AssignmentInvitationRow> = query
        .select(AssignmentInvitationRow::as_select())
        .order_by(f_id.asc())
        .offset(offset as i64)
        .limit(limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    rows_into_infos(rows)
}

async fn get_info_by_id(conn: &mut RdbConn, id: &str) -> RegularResult<AssignmentInvitationInfo> {
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

async fn get_info_by_code_excluded(
    conn: &mut RdbConn,
    code: &str,
) -> RegularResult<AssignmentInvitationInfo> {
    let row: AssignmentInvitationRow = t_assignment_invitation
        .filter(f_invitation_code.eq(code))
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

async fn create(
    conn: &mut RdbConn,
    form: &AssignmentInvitationForm,
) -> RegularResult<AssignmentInvitationInfo> {
    let entry = AssignmentInvitationEntry::from(form);

    let row: AssignmentInvitationRow = diesel::insert_into(t_assignment_invitation)
        .values(&entry)
        .returning(AssignmentInvitationRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    row_into_info(row)
}

async fn mark_pending_as_used(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
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

async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    diesel::delete(t_assignment_invitation.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &ListInfos<'a>) -> RegularResult<Vec<AssignmentInvitationInfo>> {
        submit_query!(
            self.shared,
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

    async fn execute(&self, step: &GetInfoById<'a>) -> RegularResult<AssignmentInvitationInfo> {
        submit_query!(self.shared, get_info_by_id, step.id)
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
impl<'a> Advance<GetInfoByCodeExcluded<'a>, RdbContext> for RdbRepoTransactional {
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

    async fn advance(&self, context: &mut RdbContext, step: &Delete<'a>) -> RegularResult<()> {
        delete(context.conn(), step.id).await
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;
