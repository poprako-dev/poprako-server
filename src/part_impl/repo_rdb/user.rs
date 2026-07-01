//! RDB-backed user repository — [`Execute`] and [`Advance`] implementations.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::part::repo::step::user::{
    Create, Delete, FindInfoByQid, GetCredentialByQid, GetInfoById, GetInfoExcluded,
    MarkAvatarUploaded, ReserveAvatar, TouchLastActive, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_rdb::entity::user::{UserAspect, UserCredentialRow, UserEntry, UserRow};
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional, schema};
use crate::part_impl::shared_rdb::RdbContext;
use crate::part_impl::shared_rdb::result::{diesel, expected};
use crate::result::RootError;

// ── Non-transactional ──────────────────────────────────────────────────────

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RootError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> Result<<GetInfoById<'_> as poprako_transactional::step::Step>::Output, Self::Error> {
        let mut conn = self.conn().await?;

        let row = schema::t_user::table
            .filter(schema::t_user::f_id.eq(step.id))
            .select(UserRow::as_select())
            .get_result(conn.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-user-not-found"))?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Execute<GetCredentialByQid<'a>> for RdbRepo {
    type Error = RootError;

    async fn execute(
        &self,
        step: &GetCredentialByQid<'a>,
    ) -> Result<<GetCredentialByQid<'_> as poprako_transactional::step::Step>::Output, Self::Error>
    {
        let mut conn = self.conn().await?;

        let row = schema::t_user::table
            .filter(schema::t_user::f_qid.eq(step.qid))
            .select(UserCredentialRow::as_select())
            .get_result(conn.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-user-not-found"))?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Execute<FindInfoByQid<'a>> for RdbRepo {
    type Error = RootError;

    async fn execute(
        &self,
        step: &FindInfoByQid<'a>,
    ) -> Result<<FindInfoByQid<'_> as poprako_transactional::step::Step>::Output, Self::Error> {
        let mut conn = self.conn().await?;

        let row = schema::t_user::table
            .filter(schema::t_user::f_qid.eq(step.qid))
            .select(UserRow::as_select())
            .get_result(conn.conn())
            .await
            .optional()
            .map_err(diesel)?;

        Ok(row.map(Into::into))
    }
}

// ── Transactional ──────────────────────────────────────────────────────────

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Create<'a>,
    ) -> Result<<Create<'a> as poprako_transactional::step::Step>::Output, Self::Error> {
        let now = OffsetDateTime::now_utc();

        let entry = UserEntry {
            f_id: &step.form.id,
            f_nickname: &step.form.nickname,
            f_qid: &step.form.qid,
            f_password_hash: &step.form.password_hash,
            f_last_active_at: now,
            f_created_at: now,
            f_updated_at: now,
        };

        let row = diesel::insert_into(schema::t_user::table)
            .values(&entry)
            .returning(UserRow::as_returning())
            .get_result(context.conn())
            .await
            .map_err(diesel)?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Advance<FindInfoByQid<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &FindInfoByQid<'a>,
    ) -> Result<<FindInfoByQid<'a> as poprako_transactional::step::Step>::Output, Self::Error> {
        let row = schema::t_user::table
            .filter(schema::t_user::f_qid.eq(step.qid))
            .select(UserRow::as_select())
            .get_result(context.conn())
            .await
            .optional()
            .map_err(diesel)?;

        Ok(row.map(Into::into))
    }
}

#[async_trait]
impl<'a> Advance<UpdateInfo<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &UpdateInfo<'a>,
    ) -> Result<(), RootError> {
        let now = OffsetDateTime::now_utc();

        let aspect = UserAspect::new(now).nickname(step.nickname).qid(step.qid);

        diesel::update(schema::t_user::table.filter(schema::t_user::f_id.eq(step.id)))
            .set(&aspect)
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<ReserveAvatar<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ReserveAvatar<'a>,
    ) -> Result<<ReserveAvatar<'a> as poprako_transactional::step::Step>::Output, Self::Error> {
        use crate::model::user::UserAvatarReservation;

        let now = OffsetDateTime::now_utc();

        let (prev_key, new_version): (Option<String>, i64) =
            diesel::update(schema::t_user::table.filter(schema::t_user::f_id.eq(step.id)))
                .set((
                    schema::t_user::f_avatar_key.eq::<Option<&str>>(None),
                    schema::t_user::f_avatar_uploaded.eq(false),
                    schema::t_user::f_avatar_version.eq(schema::t_user::f_avatar_version + 1),
                    schema::t_user::f_updated_at.eq(now),
                ))
                .returning((
                    schema::t_user::f_avatar_key,
                    schema::t_user::f_avatar_version,
                ))
                .get_result::<(Option<String>, i64)>(context.conn())
                .await
                .map_err(diesel)?;

        let object_key =
            crate::complex::user::UserComplex::gen_avatar_key(step.id, new_version, step.file_ext);

        Ok(UserAvatarReservation {
            object_key,
            prev_object_key: prev_key,
            avatar_version: new_version,
        })
    }
}

#[async_trait]
impl<'a> Advance<MarkAvatarUploaded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &MarkAvatarUploaded<'a>,
    ) -> Result<(), RootError> {
        let now = OffsetDateTime::now_utc();

        let affected = diesel::update(
            schema::t_user::table
                .filter(schema::t_user::f_id.eq(step.id))
                .filter(schema::t_user::f_avatar_version.eq(step.avatar_version)),
        )
        .set((
            schema::t_user::f_avatar_uploaded.eq(true),
            schema::t_user::f_updated_at.eq(now),
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
impl<'a> Advance<TouchLastActive<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &TouchLastActive<'a>,
    ) -> Result<(), RootError> {
        let now = OffsetDateTime::now_utc();

        let aspect = UserAspect::new(now).last_active_at(now);

        diesel::update(schema::t_user::table.filter(schema::t_user::f_id.eq(step.id)))
            .set(&aspect)
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoExcluded<'a>,
    ) -> Result<<GetInfoExcluded<'a> as poprako_transactional::step::Step>::Output, Self::Error>
    {
        let row = schema::t_user::table
            .filter(schema::t_user::f_id.eq(step.id))
            .select(UserRow::as_select())
            .for_update()
            .get_result(context.conn())
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-user-not-found"))?;

        Ok(row.into())
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RootError;

    async fn advance(&self, context: &mut RdbContext, step: &Delete<'a>) -> Result<(), RootError> {
        diesel::delete(schema::t_user::table.filter(schema::t_user::f_id.eq(step.id)))
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}
