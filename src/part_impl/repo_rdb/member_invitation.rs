//! RDB-backed member invitation repository — [`Execute`] and [`Advance`]
//! implementations.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::part::repo::step::member_invitation::{
    Create, Delete, GetInfoByCodeExcluded, GetInfoById, ListInfos, MarkPendingAsUsed, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_rdb::entity::member_invitation::{
    MemberInvitationAspect, MemberInvitationEntry, MemberInvitationRow,
};
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional, schema};
use crate::part_impl::shared_rdb::RdbContext;
use crate::part_impl::shared_rdb::result::{diesel, expected};
use crate::result::RegularError;

// ── Non-transactional ──────────────────────────────────────────────────────

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> Result<<ListInfos<'_> as poprako_transactional::step::Step>::Output, Self::Error> {
        let mut conn = self.conn().await?;

        let mut query = schema::t_member_invitation::table
            .filter(schema::t_member_invitation::f_team_id.eq(step.spec.team_id.as_str()))
            .select(MemberInvitationRow::as_select())
            .into_boxed();

        match step.spec.pending {
            Some(pending) => {
                query = query.filter(schema::t_member_invitation::f_pending.eq(pending));
            }
            None => {}
        }

        let rows: Vec<MemberInvitationRow> = query
            .order_by(schema::t_member_invitation::f_created_at.desc())
            .offset(step.spec.offset as i64)
            .limit(step.spec.limit as i64)
            .load(conn.conn())
            .await
            .map_err(diesel)?;

        let mut infos = Vec::with_capacity(rows.len());
        for row in rows {
            infos.push(row.try_into()?);
        }

        Ok(infos)
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> Result<<GetInfoById<'_> as poprako_transactional::step::Step>::Output, Self::Error> {
        let mut conn = self.conn().await?;

        let row = schema::t_member_invitation::table
            .filter(schema::t_member_invitation::f_id.eq(step.id))
            .select(MemberInvitationRow::as_select())
            .get_result(conn.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-invitation-not-found"))?;

        Ok(row.try_into()?)
    }
}

// ── Transactional ──────────────────────────────────────────────────────────

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Create<'a>,
    ) -> Result<<Create<'a> as poprako_transactional::step::Step>::Output, Self::Error> {
        let entry = MemberInvitationEntry::from(step.form);

        let row = diesel::insert_into(schema::t_member_invitation::table)
            .values(&entry)
            .returning(MemberInvitationRow::as_returning())
            .get_result(context.conn())
            .await
            .map_err(diesel)?;

        Ok(row.try_into()?)
    }
}

#[async_trait]
impl<'a> Advance<GetInfoByCodeExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoByCodeExcluded<'a>,
    ) -> Result<<GetInfoByCodeExcluded<'a> as poprako_transactional::step::Step>::Output, Self::Error>
    {
        let row = schema::t_member_invitation::table
            .filter(schema::t_member_invitation::f_invitation_code.eq(step.code))
            .filter(schema::t_member_invitation::f_pending.eq(true))
            .select(MemberInvitationRow::as_select())
            .for_update()
            .get_result(context.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-invitation-not-found"))?;

        Ok(row.try_into()?)
    }
}

#[async_trait]
impl<'a> Advance<MarkPendingAsUsed<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &MarkPendingAsUsed<'a>,
    ) -> Result<(), RegularError> {
        let now = OffsetDateTime::now_utc();
        let aspect = MemberInvitationAspect::new(now).pending(false);

        diesel::update(
            schema::t_member_invitation::table
                .filter(schema::t_member_invitation::f_id.eq(step.id)),
        )
        .set(&aspect)
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<GetInfoById<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoById<'a>,
    ) -> Result<<GetInfoById<'a> as poprako_transactional::step::Step>::Output, Self::Error> {
        let row = schema::t_member_invitation::table
            .filter(schema::t_member_invitation::f_id.eq(step.id))
            .select(MemberInvitationRow::as_select())
            .get_result(context.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-invitation-not-found"))?;

        Ok(row.try_into()?)
    }
}

#[async_trait]
impl<'a> Advance<UpdateInfo<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &UpdateInfo<'a>,
    ) -> Result<(), RegularError> {
        let now = OffsetDateTime::now_utc();
        let role_mask_val = i64::from(u32::from(step.update.roles));

        let aspect = MemberInvitationAspect::new(now).role_mask(role_mask_val);

        diesel::update(
            schema::t_member_invitation::table
                .filter(schema::t_member_invitation::f_id.eq(step.update.id.as_str())),
        )
        .set(&aspect)
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Delete<'a>,
    ) -> Result<(), RegularError> {
        diesel::delete(
            schema::t_member_invitation::table
                .filter(schema::t_member_invitation::f_id.eq(step.id)),
        )
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        Ok(())
    }
}
