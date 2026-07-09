//! RDB-backed assignment repository.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::model::assignment::{
    AssignmentForm, AssignmentInfo, AssignmentListSpec, AssignmentRoleUpdate,
};
use crate::part::repo::assignment::{
    AssignmentRepo, AssignmentRepoTransactional,
};
use crate::part::repo::step::assignment::{
    Create, Delete, DeleteByChapterId, GetInfoByChapterIdAndUserId,
    GetInfoById, ListAllInfosByChapter, ListInfos,
    ListInfosByChapterIdExcluded, PutRoles,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::rdb_core::RdbConn;
use crate::part_impl::rdb_core::RdbContext;
use crate::part_impl::rdb_core::result::{diesel, expected};
use crate::part_impl::repo_rdb::entity::assignment::{
    AssignmentAspect, AssignmentEntry, AssignmentRoleTimestamps, AssignmentRow,
};
use crate::part_impl::repo_rdb::incl;
use crate::part_impl::repo_rdb::{RdbRepo, RdbRepoTransactional};
use crate::result::{RegularError, RegularResult};
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::RoleField;

use crate::part_impl::repo_rdb::schema::t_assignment::dsl::*;

// FIXME: simplify: For T
impl AssignmentRepo<RdbContext> for RdbRepo {}

impl AssignmentRepoTransactional<RdbContext> for RdbRepoTransactional {}

fn row_into_info(row: AssignmentRow) -> RegularResult<AssignmentInfo> {
    row.try_into()
}

fn rows_into_infos(
    rows: Vec<AssignmentRow>,
) -> RegularResult<Vec<AssignmentInfo>> {
    rows.into_iter().map(row_into_info).collect()
}

async fn get_info_by_chapter_id_and_user_id(
    conn: &mut RdbConn,
    chapter_id: &str,
    user_id: &str,
) -> RegularResult<Option<AssignmentInfo>> {
    //
    let row: Option<AssignmentRow> = t_assignment
        .filter(f_chapter_id.eq(chapter_id))
        .filter(f_user_id.eq(user_id))
        .select(AssignmentRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    row.map(row_into_info).transpose()
}

async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[AssignmentInclOpt],
) -> RegularResult<AssignmentInfo> {
    //
    let row: AssignmentRow = t_assignment
        .filter(f_id.eq(id))
        .select(AssignmentRow::as_select())
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?
        .ok_or_else(|| expected("error-assignment-not-found"))?;

    let mut info = row_into_info(row)?;

    incl::assignment::populate_assignment_incls(
        conn,
        std::slice::from_mut(&mut info),
        incl_opt,
    )
    .await?;

    Ok(info)
}

async fn list_infos(
    conn: &mut RdbConn,
    spec: &AssignmentListSpec,
) -> RegularResult<Vec<AssignmentInfo>> {
    //
    let (role, incl_opt, offset, limit, mut query) = match spec {
        AssignmentListSpec::Chapter {
            chapter_id,
            role,
            incl_opt,
            offset,
            limit,
        } => (
            role,
            incl_opt,
            offset,
            limit,
            t_assignment
                .filter(f_chapter_id.eq(chapter_id.as_str()))
                .into_boxed(),
        ),
        AssignmentListSpec::User {
            owner_id,
            role,
            incl_opt,
            offset,
            limit,
        } => (
            role,
            incl_opt,
            offset,
            limit,
            t_assignment
                .filter(f_user_id.eq(owner_id.as_str()))
                .into_boxed(),
        ),
    };

    if let Some(role) = role {
        query = match *role {
            RoleField::RAW_PROVIDER => {
                query.filter(f_assigned_raw_provider_at.is_not_null())
            }
            RoleField::TRANSLATOR => {
                query.filter(f_assigned_translator_at.is_not_null())
            }
            RoleField::PROOFREADER => {
                query.filter(f_assigned_proofreader_at.is_not_null())
            }
            RoleField::TYPESETTER => {
                query.filter(f_assigned_typesetter_at.is_not_null())
            }
            RoleField::REDRAWER => {
                query.filter(f_assigned_redrawer_at.is_not_null())
            }
            RoleField::REVIEWER => {
                query.filter(f_assigned_reviewer_at.is_not_null())
            }
            RoleField::PUBLISHER => {
                query.filter(f_assigned_publisher_at.is_not_null())
            }
            RoleField::ADMIN => query.filter(f_assigned_admin_at.is_not_null()),
            _ => query,
        };
    }

    let rows: Vec<AssignmentRow> = query
        .select(AssignmentRow::as_select())
        .order_by((f_created_at.desc(), f_id.asc()))
        .offset(*offset as i64)
        .limit(*limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    let mut infos = rows_into_infos(rows)?;

    incl::assignment::populate_assignment_incls(conn, &mut infos, incl_opt)
        .await?;

    Ok(infos)
}

async fn list_all_infos_by_chapter(
    conn: &mut RdbConn,
    chapter_id: &str,
    role: Option<RoleField>,
    incl_opt: &[AssignmentInclOpt],
) -> RegularResult<Vec<AssignmentInfo>> {
    //
    let mut query = t_assignment
        .filter(f_chapter_id.eq(chapter_id))
        .into_boxed();

    if let Some(role) = role {
        query = match role {
            RoleField::RAW_PROVIDER => {
                query.filter(f_assigned_raw_provider_at.is_not_null())
            }
            RoleField::TRANSLATOR => {
                query.filter(f_assigned_translator_at.is_not_null())
            }
            RoleField::PROOFREADER => {
                query.filter(f_assigned_proofreader_at.is_not_null())
            }
            RoleField::TYPESETTER => {
                query.filter(f_assigned_typesetter_at.is_not_null())
            }
            RoleField::REDRAWER => {
                query.filter(f_assigned_redrawer_at.is_not_null())
            }
            RoleField::REVIEWER => {
                query.filter(f_assigned_reviewer_at.is_not_null())
            }
            RoleField::PUBLISHER => {
                query.filter(f_assigned_publisher_at.is_not_null())
            }
            RoleField::ADMIN => query.filter(f_assigned_admin_at.is_not_null()),
            _ => query,
        };
    }

    let rows: Vec<AssignmentRow> = query
        .select(AssignmentRow::as_select())
        .order_by((f_created_at.desc(), f_id.asc()))
        .load(conn)
        .await
        .map_err(diesel)?;

    let mut infos = rows_into_infos(rows)?;

    incl::assignment::populate_assignment_incls(conn, &mut infos, incl_opt)
        .await?;

    Ok(infos)
}

async fn list_infos_by_chapter_id_excluded(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> RegularResult<Vec<AssignmentInfo>> {
    //
    let rows: Vec<AssignmentRow> = t_assignment
        .filter(f_chapter_id.eq(chapter_id))
        .select(AssignmentRow::as_select())
        .order_by((f_created_at.desc(), f_id.asc()))
        .for_update()
        .load(conn)
        .await
        .map_err(diesel)?;

    rows_into_infos(rows)
}

async fn create(
    conn: &mut RdbConn,
    form: &AssignmentForm,
) -> RegularResult<AssignmentInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let entry = AssignmentEntry::from_form(form, now);

    let row: AssignmentRow = diesel::insert_into(t_assignment)
        .values(&entry)
        .returning(AssignmentRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    row_into_info(row)
}

async fn put_roles(
    conn: &mut RdbConn,
    update: &AssignmentRoleUpdate,
) -> RegularResult<AssignmentInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let timestamps = AssignmentRoleTimestamps::from_mask(update.roles, now);

    let aspect = AssignmentAspect::new(now).roles(timestamps);

    let row: AssignmentRow =
        diesel::update(t_assignment.filter(f_id.eq(update.id.as_str())))
            .set(&aspect)
            .returning(AssignmentRow::as_returning())
            .get_result(conn)
            .await
            .optional()
            .map_err(diesel)?
            .ok_or_else(|| expected("error-assignment-not-found"))?;

    row_into_info(row)
}

async fn delete(conn: &mut RdbConn, id: &str) -> RegularResult<()> {
    //
    diesel::delete(t_assignment.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

async fn delete_by_chapter_id(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> RegularResult<()> {
    //
    diesel::delete(t_assignment.filter(f_chapter_id.eq(chapter_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    Ok(())
}

#[async_trait]
impl<'a> Execute<GetInfoByChapterIdAndUserId<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoByChapterIdAndUserId<'a>,
    ) -> RegularResult<Option<AssignmentInfo>> {
        submit_query!(
            self.core,
            get_info_by_chapter_id_and_user_id,
            step.chapter_id,
            step.user_id
        )
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> RegularResult<Vec<AssignmentInfo>> {
        submit_query!(self.core, list_infos, step.spec)
    }
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> RegularResult<AssignmentInfo> {
        submit_query!(self.core, get_info_by_id, step.id, step.incl_opt)
    }
}

#[async_trait]
impl<'a> Execute<ListAllInfosByChapter<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListAllInfosByChapter<'a>,
    ) -> RegularResult<Vec<AssignmentInfo>> {
        submit_query!(
            self.core,
            list_all_infos_by_chapter,
            step.chapter_id,
            step.role,
            step.incl_opt
        )
    }
}

#[async_trait]
impl<'a> Advance<ListAllInfosByChapter<'a>, RdbContext>
    for RdbRepoTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ListAllInfosByChapter<'a>,
    ) -> RegularResult<Vec<AssignmentInfo>> {
        list_all_infos_by_chapter(
            context.conn(),
            step.chapter_id,
            step.role,
            step.incl_opt,
        )
        .await
    }
}

#[async_trait]
impl<'a> Advance<GetInfoByChapterIdAndUserId<'a>, RdbContext>
    for RdbRepoTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &GetInfoByChapterIdAndUserId<'a>,
    ) -> RegularResult<Option<AssignmentInfo>> {
        get_info_by_chapter_id_and_user_id(
            context.conn(),
            step.chapter_id,
            step.user_id,
        )
        .await
    }
}

#[async_trait]
impl<'a> Advance<ListInfosByChapterIdExcluded<'a>, RdbContext>
    for RdbRepoTransactional
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ListInfosByChapterIdExcluded<'a>,
    ) -> RegularResult<Vec<AssignmentInfo>> {
        list_infos_by_chapter_id_excluded(context.conn(), step.chapter_id).await
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Create<'a>,
    ) -> RegularResult<AssignmentInfo> {
        create(context.conn(), step.form).await
    }
}

#[async_trait]
impl<'a> Advance<PutRoles<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &PutRoles<'a>,
    ) -> RegularResult<AssignmentInfo> {
        put_roles(context.conn(), step.update).await
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
