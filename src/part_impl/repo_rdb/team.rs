//! RDB-backed team repository — free query functions and thin trait impls.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::complex::team::TeamComplex;
use crate::model::team::{TeamAvatarReservation, TeamForm, TeamInfo};
use crate::part::repo::step::team::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrementWorksetNextIndex, ListInfos,
    MarkAvatarUploaded, ReserveAvatar, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_rdb::entity::team::{TeamAspect, TeamEntry, TeamRow};
use crate::part_impl::repo_rdb::schema::t_member;
use crate::part_impl::repo_rdb::schema::t_team::dsl::*;
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional};
use crate::part_impl::shared_rdb::result::{diesel, expected};
use crate::part_impl::shared_rdb::{RdbConn, RdbContext};
use crate::result::{RegularError, RegularResult};

// ── Free functions ──────────────────────────────────────────────────────────

async fn create(conn: &mut RdbConn, form: &TeamForm) -> RegularResult<TeamInfo> {
    let now = OffsetDateTime::now_utc();

    let entry = TeamEntry {
        f_id: &form.id,
        f_name: &form.name,
        f_description: &form.description,
        f_workset_next_index: 0,
        f_created_at: now,
        f_updated_at: now,
    };

    let row: TeamRow = diesel::insert_into(t_team)
        .values(&entry)
        .returning(TeamRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(row.into())
}

async fn get_info_by_id(conn: &mut RdbConn, id: &str) -> RegularResult<TeamInfo> {
    let row: TeamRow = t_team
        .filter(f_id.eq(id))
        .select(TeamRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-team-not-found"))?;

    Ok(row.into())
}

async fn list_infos(
    conn: &mut RdbConn,
    user_id: Option<&str>,
    offset: u64,
    limit: u64,
) -> RegularResult<Vec<TeamInfo>> {
    let mut query = t_team.into_boxed();

    if let Some(user_id) = user_id {
        let member_team_ids = t_member::table
            .filter(t_member::f_user_id.eq(user_id))
            .select(t_member::f_team_id);
        query = query.filter(f_id.eq_any(member_team_ids));
    }

    let rows: Vec<TeamRow> = query
        .select(TeamRow::as_select())
        .order_by(f_created_at.desc())
        .offset(offset as i64)
        .limit(limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

async fn update_info(
    conn: &mut RdbConn,
    id: &str,
    name: &str,
    description: &str,
) -> RegularResult<()> {
    let now = OffsetDateTime::now_utc();

    let aspect = TeamAspect::new(now).name(name).description(description);

    diesel::update(t_team.filter(f_id.eq(id)))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn mark_avatar_uploaded(conn: &mut RdbConn, id: &str, version: i64) -> RegularResult<()> {
    let now = OffsetDateTime::now_utc();

    let affected = diesel::update(
        t_team
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

async fn reserve_avatar(
    conn: &mut RdbConn,
    id: &str,
    file_ext: &str,
) -> RegularResult<TeamAvatarReservation> {
    let now = OffsetDateTime::now_utc();

    let (prev_key, new_version): (Option<String>, i64) = diesel::update(t_team.filter(f_id.eq(id)))
        .set((
            f_avatar_key.eq::<Option<&str>>(None),
            f_avatar_uploaded.eq(false),
            f_avatar_version.eq(f_avatar_version + 1),
            f_updated_at.eq(now),
        ))
        .returning((f_avatar_key, f_avatar_version))
        .get_result(conn)
        .await
        .map_err(diesel)?;

    let object_key = TeamComplex::gen_avatar_key(id, new_version, file_ext);

    Ok(TeamAvatarReservation {
        object_key,
        prev_object_key: prev_key,
        avatar_version: new_version,
    })
}

async fn get_info_excluded(conn: &mut RdbConn, id: &str) -> RegularResult<TeamInfo> {
    let row: TeamRow = t_team
        .filter(f_id.eq(id))
        .select(TeamRow::as_select())
        .for_update()
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-team-not-found"))?;

    Ok(row.into())
}

async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    diesel::delete(t_team.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn increment_workset_next_index(conn: &mut RdbConn, id: &str) -> RegularResult<i32> {
    let prev: i32 = diesel::update(t_team.filter(f_id.eq(id)))
        .set(f_workset_next_index.eq(f_workset_next_index + 1))
        .returning(f_workset_next_index - 1)
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(prev)
}

// ── Non-transactional: Execute impls ─────────────────────────────────────────

#[async_trait]
impl<'a> Execute<Create<'a>> for RdbRepo {
    type Error = RegularError;
    async fn execute(&self, step: &Create<'a>) -> Result<TeamInfo, Self::Error> {
        submit_query!(self.shared, create, step.form)
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;
    async fn execute(&self, step: &GetInfoById<'a>) -> Result<TeamInfo, Self::Error> {
        submit_query!(self.shared, get_info_by_id, step.id)
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;
    async fn execute(&self, step: &ListInfos<'a>) -> Result<Vec<TeamInfo>, Self::Error> {
        submit_query!(
            self.shared,
            list_infos,
            step.user_id,
            step.offset,
            step.limit
        )
    }
}

#[async_trait]
impl<'a> Execute<UpdateInfo<'a>> for RdbRepo {
    type Error = RegularError;
    async fn execute(&self, step: &UpdateInfo<'a>) -> RegularResult<()> {
        submit_query!(
            self.shared,
            update_info,
            step.id,
            step.name,
            step.description
        )
    }
}

#[async_trait]
impl<'a> Execute<MarkAvatarUploaded<'a>> for RdbRepo {
    type Error = RegularError;
    async fn execute(&self, step: &MarkAvatarUploaded<'a>) -> RegularResult<()> {
        submit_query!(
            self.shared,
            mark_avatar_uploaded,
            step.id,
            step.avatar_version
        )
    }
}

// ── Transactional: Advance impls ─────────────────────────────────────────────

#[async_trait]
impl<'a> Advance<ReserveAvatar<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;
    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ReserveAvatar<'a>,
    ) -> RegularResult<TeamAvatarReservation> {
        reserve_avatar(context.conn(), step.id, step.file_extension).await
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
impl<'a> Advance<GetInfoExcluded<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;
    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoExcluded<'a>,
    ) -> RegularResult<TeamInfo> {
        get_info_excluded(context.conn(), step.id).await
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;
    async fn advance(&self, context: &mut RdbContext, step: &Delete<'a>) -> RegularResult<()> {
        delete(context.conn(), step.id).await
    }
}

#[async_trait]
impl<'a> Advance<IncrementWorksetNextIndex<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;
    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &IncrementWorksetNextIndex<'a>,
    ) -> RegularResult<i32> {
        increment_workset_next_index(context.conn(), step.id).await
    }
}
