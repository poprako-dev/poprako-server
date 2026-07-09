//! RDB-backed user repository — free query functions and thin trait impls.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::complex::user::UserComplex;
use crate::model::user::{
    UserAvatarReservation, UserCredential, UserForm, UserInfo,
};
use crate::part::repo::step::user::{
    Create, Delete, FindInfoByQid, GetCredentialByQid, GetInfoById,
    GetInfoExcluded, MarkAvatarUploaded, ReserveAvatar, TouchLastActive,
    UpdateInfo,
};
use crate::part::repo::user::{UserRepo, UserRepoTransactional};
use crate::part::shared::execute::Execute;
use crate::part_impl::shared::RdbConn;
use crate::part_impl::shared::RdbContext;
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::repo::rdb_impl::entity::user::{
    UserAspect, UserCredentialRow, UserEntry, UserRow,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, RdbRepoTransactional};
use crate::result::{RegularError, RegularResult};

use crate::part_impl::repo::rdb_impl::schema::t_user::dsl::*;

impl UserRepo<RdbContext> for RdbRepo {}

impl UserRepoTransactional<RdbContext> for RdbRepoTransactional {}

// ── Free functions ──────────────────────────────────────────────────────────

async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<UserInfo> {
    //
    let row: UserRow = t_user
        .filter(f_id.eq(id))
        .select(UserRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-user-not-found"))?;

    Ok(row.into())
}

async fn get_credential_by_qid(
    conn: &mut RdbConn,
    qid: &str,
) -> RegularResult<UserCredential> {
    //
    let row: UserCredentialRow = t_user
        .filter(f_qid.eq(qid))
        .select(UserCredentialRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-user-not-found"))?;

    Ok(row.into())
}

async fn find_info_by_qid(
    conn: &mut RdbConn,
    qid: &str,
) -> RegularResult<Option<UserInfo>> {
    //
    let row: Option<UserRow> = t_user
        .filter(f_qid.eq(qid))
        .select(UserRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    Ok(row.map(Into::into))
}

async fn create(
    conn: &mut RdbConn,
    form: &UserForm,
) -> RegularResult<UserInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let entry = UserEntry {
        f_id: &form.id,
        f_nickname: &form.nickname,
        f_qid: &form.qid,
        f_password_hash: &form.password_hash,
        f_last_active_at: now,
        f_created_at: now,
        f_updated_at: now,
    };

    let row: UserRow = diesel::insert_into(t_user)
        .values(&entry)
        .returning(UserRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(row.into())
}

async fn update_info(
    conn: &mut RdbConn,
    id: &str,
    qid: &str,
    nickname: &str,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = UserAspect::new(now).nickname(nickname).qid(qid);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn reserve_avatar(
    conn: &mut RdbConn,
    id: &str,
    file_ext: &str,
) -> RegularResult<UserAvatarReservation> {
    //
    let now = OffsetDateTime::now_utc();

    let (prev_key, new_version): (Option<String>, i64) =
        diesel::update(t_user.filter(f_id.eq(id)))
            .set((
                f_avatar_key.eq::<Option<&str>>(None),
                f_avatar_uploaded.eq(false),
                f_avatar_version.eq(f_avatar_version + 1),
                f_updated_at.eq(now),
            ))
            .returning((f_avatar_key, f_avatar_version))
            .get_result::<(Option<String>, i64)>(conn)
            .await
            .map_err(diesel)?;

    let object_key = UserComplex::gen_avatar_key(id, new_version, file_ext);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set((f_avatar_key.eq(Some(&object_key)), f_updated_at.eq(now)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(UserAvatarReservation {
        object_key,
        prev_object_key: prev_key,
        avatar_version: new_version,
    })
}

async fn mark_avatar_uploaded(
    conn: &mut RdbConn,
    id: &str,
    version: i64,
) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let affected = diesel::update(
        t_user
            .filter(f_id.eq(id))
            .filter(f_avatar_version.eq(version)),
    )
    .set((f_avatar_uploaded.eq(true), f_updated_at.eq(now)))
    .execute(conn)
    .await
    .map_err(diesel)?;

    if affected == 0 {
        return Err(expected("error-avatar-version-mismatch"));
    }

    Ok(())
}

async fn touch_last_active(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    //
    let now = OffsetDateTime::now_utc();

    let aspect = UserAspect::new(now).last_active_at(now);

    diesel::update(t_user.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn get_info_by_id_excluded(
    conn: &mut RdbConn,
    id: &str,
) -> RegularResult<UserInfo> {
    //
    let row: UserRow = t_user
        .filter(f_id.eq(id))
        .select(UserRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-user-not-found"))?;

    Ok(row.into())
}

async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    //
    diesel::delete(t_user.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

// ── Non-transactional: Execute impls ─────────────────────────────────────────

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> Result<UserInfo, Self::Error> {
        submit_query!(self.core, get_info_by_id, step.id)
    }
}

#[async_trait]
impl<'a> Execute<GetCredentialByQid<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetCredentialByQid<'a>,
    ) -> Result<UserCredential, Self::Error> {
        submit_query!(self.core, get_credential_by_qid, step.qid)
    }
}

#[async_trait]
impl<'a> Execute<FindInfoByQid<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &FindInfoByQid<'a>,
    ) -> Result<Option<UserInfo>, Self::Error> {
        submit_query!(self.core, find_info_by_qid, step.qid)
    }
}

#[async_trait]
impl<'a> Execute<TouchLastActive<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, step: &TouchLastActive<'a>) -> RegularResult<()> {
        submit_query!(self.core, touch_last_active, step.id)
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
    ) -> RegularResult<UserInfo> {
        create(context.conn(), step.form).await
    }
}

#[async_trait]
impl<'a> Advance<FindInfoByQid<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &FindInfoByQid<'a>,
    ) -> RegularResult<Option<UserInfo>> {
        find_info_by_qid(context.conn(), step.qid).await
    }
}

#[async_trait]
impl<'a> Advance<UpdateInfo<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &UpdateInfo<'a>,
    ) -> RegularResult<()> {
        update_info(context.conn(), step.id, step.qid, step.nickname).await
    }
}

#[async_trait]
impl<'a> Advance<ReserveAvatar<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ReserveAvatar<'a>,
    ) -> RegularResult<UserAvatarReservation> {
        reserve_avatar(context.conn(), step.id, step.file_ext).await
    }
}

#[async_trait]
impl<'a> Advance<MarkAvatarUploaded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &MarkAvatarUploaded<'a>,
    ) -> RegularResult<()> {
        mark_avatar_uploaded(context.conn(), step.id, step.avatar_version).await
    }
}

#[async_trait]
impl<'a> Advance<TouchLastActive<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &TouchLastActive<'a>,
    ) -> RegularResult<()> {
        touch_last_active(context.conn(), step.id).await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoExcluded<'a>,
    ) -> RegularResult<UserInfo> {
        get_info_by_id_excluded(context.conn(), step.id).await
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
#[cfg(all(test, feature = "repo"))]
mod tests;
