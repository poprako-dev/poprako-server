//! RDB-backed workset repository — [`Execute`] and [`Advance`] implementations.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::part::repo::step::workset::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrComicNextIndex, ListInfosByTeamId,
    ListInfosByTeamIdExcluded, UpdateComicCount, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_rdb::entity::workset::{WorksetAspect, WorksetEntry, WorksetRow};
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional, schema};
use crate::part_impl::shared_rdb::RdbContext;
use crate::part_impl::shared_rdb::result::{diesel, expected};
use crate::result::RegularError;

// ── Non-transactional ──────────────────────────────────────────────────────

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> Result<<GetInfoById<'_> as poprako_transactional::step::Step>::Output, Self::Error> {
        let mut conn = self.conn().await?;

        let row = schema::t_workset::table
            .filter(schema::t_workset::f_id.eq(step.id))
            .select(WorksetRow::as_select())
            .get_result(conn.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-workset-not-found"))?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Execute<ListInfosByTeamId<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfosByTeamId<'a>,
    ) -> Result<<ListInfosByTeamId<'_> as poprako_transactional::step::Step>::Output, Self::Error>
    {
        let mut conn = self.conn().await?;

        let rows: Vec<WorksetRow> = schema::t_workset::table
            .filter(schema::t_workset::f_team_id.eq(step.team_id))
            .select(WorksetRow::as_select())
            .order_by(schema::t_workset::f_index.asc())
            .offset(step.offset as i64)
            .limit(step.limit as i64)
            .load(conn.conn())
            .await
            .map_err(diesel)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[async_trait]
impl<'a> Execute<UpdateInfo<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &UpdateInfo<'a>) -> Result<(), RegularError> {
        let mut conn = self.conn().await?;
        let now = OffsetDateTime::now_utc();

        let aspect = WorksetAspect::new(now)
            .name(&step.update.name)
            .description(step.update.description.as_deref());

        diesel::update(
            schema::t_workset::table.filter(schema::t_workset::f_id.eq(step.update.id.as_str())),
        )
        .set(&aspect)
        .execute(conn.conn())
        .await
        .map_err(diesel)?;

        Ok(())
    }
}

// ── Transactional ──────────────────────────────────────────────────────────

#[async_trait]
impl<'a> Advance<ListInfosByTeamIdExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ListInfosByTeamIdExcluded<'a>,
    ) -> Result<
        <ListInfosByTeamIdExcluded<'a> as poprako_transactional::step::Step>::Output,
        Self::Error,
    > {
        let rows: Vec<WorksetRow> = schema::t_workset::table
            .filter(schema::t_workset::f_team_id.eq(step.team_id))
            .select(WorksetRow::as_select())
            .for_update()
            .load(context.conn())
            .await
            .map_err(diesel)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoExcluded<'a>,
    ) -> Result<<GetInfoExcluded<'a> as poprako_transactional::step::Step>::Output, Self::Error>
    {
        let row = schema::t_workset::table
            .filter(schema::t_workset::f_id.eq(step.id))
            .select(WorksetRow::as_select())
            .for_update()
            .get_result(context.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-workset-not-found"))?;

        Ok(row.into())
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
        diesel::delete(schema::t_workset::table.filter(schema::t_workset::f_id.eq(step.id)))
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
        let row = schema::t_workset::table
            .filter(schema::t_workset::f_id.eq(step.id))
            .select(WorksetRow::as_select())
            .get_result(context.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-workset-not-found"))?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Create<'a>,
    ) -> Result<<Create<'a> as poprako_transactional::step::Step>::Output, Self::Error> {
        let entry = WorksetEntry::from(step.form);

        let row = diesel::insert_into(schema::t_workset::table)
            .values(&entry)
            .returning(WorksetRow::as_returning())
            .get_result(context.conn())
            .await
            .map_err(diesel)?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Advance<IncrComicNextIndex<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &IncrComicNextIndex<'a>,
    ) -> Result<<IncrComicNextIndex<'a> as poprako_transactional::step::Step>::Output, Self::Error>
    {
        let prev: i32 =
            diesel::update(schema::t_workset::table.filter(schema::t_workset::f_id.eq(step.id)))
                .set(
                    schema::t_workset::f_comic_next_index
                        .eq(schema::t_workset::f_comic_next_index + 1),
                )
                .returning(schema::t_workset::f_comic_next_index - 1)
                .get_result(context.conn())
                .await
                .map_err(diesel)?;

        Ok(prev)
    }
}

#[async_trait]
impl<'a> Advance<UpdateComicCount<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &UpdateComicCount<'a>,
    ) -> Result<(), RegularError> {
        diesel::update(schema::t_workset::table.filter(schema::t_workset::f_id.eq(step.id)))
            .set(schema::t_workset::f_comic_count.eq(schema::t_workset::f_comic_count + step.delta))
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}
