//! RDB-backed member-invitation repository — free query functions and thin trait impls.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::model::member_invitation::{
    MemberInvitationForm, MemberInvitationInfo, MemberInvitationListSpec,
};
use crate::part::repo::member_invitation::{
    MemberInvitationRepo, MemberInvitationRepoTransactional,
};
use crate::part::repo::step::member_invitation::{
    Create, Delete, GetInfoByCodeExcluded, GetInfoById, ListInfos, MarkPendingAsUsed, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::rdb_core::result::{diesel, expected};
use crate::part_impl::rdb_core::{RdbConn, RdbContext};
use crate::part_impl::repo_rdb::entity::member_invitation::{
    MemberInvitationAspect, MemberInvitationEntry, MemberInvitationRow,
};
use crate::part_impl::repo_rdb::schema::t_member_invitation::dsl::*;
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional, incl};
use crate::result::{RegularError, RegularResult};
use crate::value::member_invitation::MemberInvitationInclOpt;
use crate::value::role::RoleMask;

impl MemberInvitationRepo<RdbContext> for RdbRepo {}

impl MemberInvitationRepoTransactional<RdbContext> for RdbRepoTransactional {}

// ── Free functions ──────────────────────────────────────────────────────────

async fn list_infos(
    conn: &mut RdbConn,
    spec: &MemberInvitationListSpec,
) -> RegularResult<Vec<MemberInvitationInfo>> {
    let mut query = t_member_invitation
        .filter(f_team_id.eq(spec.team_id.as_str()))
        .select(MemberInvitationRow::as_select())
        .into_boxed();

    if let Some(is_pending) = spec.pending {
        query = query.filter(f_pending.eq(is_pending));
    }

    let rows: Vec<MemberInvitationRow> = query
        .order_by(f_created_at.desc())
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    let mut infos = Vec::with_capacity(rows.len());

    for row in rows {
        infos.push(row.try_into()?);
    }

    incl::member_invitation::populate_member_invitation_incls(conn, &mut infos, &spec.incl_opt)
        .await?;

    Ok(infos)
}

async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[MemberInvitationInclOpt],
) -> RegularResult<MemberInvitationInfo> {
    let row: MemberInvitationRow = t_member_invitation
        .filter(f_id.eq(id))
        .select(MemberInvitationRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-invitation-not-found"))?;

    let mut info: MemberInvitationInfo = row.try_into()?;

    incl::member_invitation::populate_member_invitation_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    Ok(info)
}

async fn create(
    conn: &mut RdbConn,
    form: &MemberInvitationForm,
) -> RegularResult<MemberInvitationInfo> {
    let entry = MemberInvitationEntry::from(form);

    let row: MemberInvitationRow = diesel::insert_into(t_member_invitation)
        .values(&entry)
        .returning(MemberInvitationRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    row.try_into()
}

async fn get_info_by_code_excluded(
    conn: &mut RdbConn,
    code: &str,
) -> RegularResult<MemberInvitationInfo> {
    let row: MemberInvitationRow = t_member_invitation
        .filter(f_code.eq(code))
        .filter(f_pending.eq(true))
        .select(MemberInvitationRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-invitation-not-found"))?;

    row.try_into()
}

async fn mark_pending_as_used(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    let now = OffsetDateTime::now_utc();

    let aspect = MemberInvitationAspect::new(now).pending(false);

    diesel::update(t_member_invitation.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn update_info(conn: &mut RdbConn, id: &str, roles: RoleMask) -> RegularResult<()> {
    let now = OffsetDateTime::now_utc();

    let aspect = MemberInvitationAspect::new(now).role_mask(i64::from(u32::from(roles)));

    diesel::update(t_member_invitation.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    diesel::delete(t_member_invitation.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

// ── Non-transactional: Execute impls ─────────────────────────────────────────

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &ListInfos<'a>) -> RegularResult<Vec<MemberInvitationInfo>> {
        submit_query!(self.core, list_infos, step.spec)
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &GetInfoById<'a>) -> RegularResult<MemberInvitationInfo> {
        submit_query!(self.core, get_info_by_id, step.id, step.incl_opt)
    }
}

// ── Transactional: Advance impls ─────────────────────────────────────────────

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Create<'a>,
    ) -> RegularResult<MemberInvitationInfo> {
        create(context.conn(), step.form).await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoByCodeExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoByCodeExcluded<'a>,
    ) -> RegularResult<MemberInvitationInfo> {
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
impl<'a> Advance<GetInfoById<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoById<'a>,
    ) -> RegularResult<MemberInvitationInfo> {
        get_info_by_id(context.conn(), step.id, step.incl_opt).await
    }
}

#[async_trait]
impl<'a> Advance<UpdateInfo<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(&self, context: &mut RdbContext, step: &UpdateInfo<'a>) -> RegularResult<()> {
        update_info(context.conn(), step.update.id.as_str(), step.update.roles).await
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
