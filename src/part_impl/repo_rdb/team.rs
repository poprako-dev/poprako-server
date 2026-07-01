//! RDB-backed team repository — [`Execute`] and [`Advance`] implementations.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::model::team::TeamAvatarReservation;
use crate::part::repo::step::team::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrementWorksetNextIndex, ListInfos,
    MarkAvatarUploaded, ReserveAvatar, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_rdb::entity::team::{TeamAspect, TeamEntry, TeamRow};
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional, schema};
use crate::part_impl::shared_rdb::RdbContext;
use crate::part_impl::shared_rdb::result::{diesel, expected};
use crate::result::RegularError;

// ── Non-transactional ──────────────────────────────────────────────────────

#[async_trait]
impl<'a> Execute<Create<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &Create<'a>,
    ) -> Result<<Create<'_> as poprako_transactional::step::Step>::Output, Self::Error> {
        let mut conn = self.conn().await?;
        let now = OffsetDateTime::now_utc();

        let entry = TeamEntry {
            f_id: &step.form.id,
            f_name: &step.form.name,
            f_description: &step.form.description,
            f_workset_next_index: 0,
            f_created_at: now,
            f_updated_at: now,
        };

        let row = diesel::insert_into(schema::t_team::table)
            .values(&entry)
            .returning(TeamRow::as_returning())
            .get_result(conn.conn())
            .await
            .map_err(diesel)?;

        Ok(row.into())
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

        let row = schema::t_team::table
            .filter(schema::t_team::f_id.eq(step.id))
            .select(TeamRow::as_select())
            .get_result(conn.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-team-not-found"))?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> Result<<ListInfos<'_> as poprako_transactional::step::Step>::Output, Self::Error> {
        let mut conn = self.conn().await?;

        let mut query = schema::t_team::table.into_boxed();

        match step.user_id {
            Some(user_id) => {
                let member_team_ids = schema::t_member::table
                    .filter(schema::t_member::f_user_id.eq(user_id))
                    .select(schema::t_member::f_team_id);
                query = query.filter(schema::t_team::f_id.eq_any(member_team_ids));
            }
            None => {}
        }

        let rows: Vec<TeamRow> = query
            .select(TeamRow::as_select())
            .order_by(schema::t_team::f_created_at.desc())
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

        let aspect = TeamAspect::new(now)
            .name(step.name)
            .description(step.description);

        diesel::update(schema::t_team::table.filter(schema::t_team::f_id.eq(step.id)))
            .set(&aspect)
            .execute(conn.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Execute<MarkAvatarUploaded<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &MarkAvatarUploaded<'a>) -> Result<(), RegularError> {
        let mut conn = self.conn().await?;
        let now = OffsetDateTime::now_utc();

        let affected = diesel::update(
            schema::t_team::table
                .filter(schema::t_team::f_id.eq(step.id))
                .filter(schema::t_team::f_avatar_version.eq(step.avatar_version)),
        )
        .set((
            schema::t_team::f_avatar_uploaded.eq(true),
            schema::t_team::f_updated_at.eq(now),
        ))
        .execute(conn.conn())
        .await
        .map_err(diesel)?;

        if affected == 0 {
            return Err(expected("error-avatar-version-mismatch"));
        }

        Ok(())
    }
}

// ── Transactional ──────────────────────────────────────────────────────────

#[async_trait]
impl<'a> Advance<ReserveAvatar<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ReserveAvatar<'a>,
    ) -> Result<<ReserveAvatar<'a> as poprako_transactional::step::Step>::Output, Self::Error> {
        let now = OffsetDateTime::now_utc();

        let (prev_key, new_version): (Option<String>, i64) =
            diesel::update(schema::t_team::table.filter(schema::t_team::f_id.eq(step.id)))
                .set((
                    schema::t_team::f_avatar_key.eq::<Option<&str>>(None),
                    schema::t_team::f_avatar_uploaded.eq(false),
                    schema::t_team::f_avatar_version.eq(schema::t_team::f_avatar_version + 1),
                    schema::t_team::f_updated_at.eq(now),
                ))
                .returning((
                    schema::t_team::f_avatar_key,
                    schema::t_team::f_avatar_version,
                ))
                .get_result::<(Option<String>, i64)>(context.conn())
                .await
                .map_err(diesel)?;

        let object_key = crate::complex::team::TeamComplex::gen_avatar_key(
            step.id,
            new_version,
            step.file_extension,
        );

        Ok(TeamAvatarReservation {
            object_key,
            prev_object_key: prev_key,
            avatar_version: new_version,
        })
    }
}

#[async_trait]
impl<'a> Advance<MarkAvatarUploaded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &MarkAvatarUploaded<'a>,
    ) -> Result<(), RegularError> {
        let now = OffsetDateTime::now_utc();

        let affected = diesel::update(
            schema::t_team::table
                .filter(schema::t_team::f_id.eq(step.id))
                .filter(schema::t_team::f_avatar_version.eq(step.avatar_version)),
        )
        .set((
            schema::t_team::f_avatar_uploaded.eq(true),
            schema::t_team::f_updated_at.eq(now),
        ))
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        if affected == 0 {
            return Err(expected("error-avatar-version-mismatch"));
        }

        Ok(())
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
        let row = schema::t_team::table
            .filter(schema::t_team::f_id.eq(step.id))
            .select(TeamRow::as_select())
            .for_update()
            .get_result(context.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-team-not-found"))?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(&self, context: &mut RdbContext, step: &Delete<'a>) -> Result<(), RegularError> {
        diesel::delete(schema::t_team::table.filter(schema::t_team::f_id.eq(step.id)))
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<IncrementWorksetNextIndex<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &IncrementWorksetNextIndex<'a>,
    ) -> Result<
        <IncrementWorksetNextIndex<'a> as poprako_transactional::step::Step>::Output,
        Self::Error,
    > {
        let prev: i32 =
            diesel::update(schema::t_team::table.filter(schema::t_team::f_id.eq(step.id)))
                .set(
                    schema::t_team::f_workset_next_index
                        .eq(schema::t_team::f_workset_next_index + 1),
                )
                .returning(schema::t_team::f_workset_next_index - 1)
                .get_result(context.conn())
                .await
                .map_err(diesel)?;

        Ok(prev)
    }
}
