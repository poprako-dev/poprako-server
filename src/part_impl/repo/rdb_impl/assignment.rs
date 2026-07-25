//! RDB-backed assignment repository.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use self::list::list_infos;
use crate::model::assignment::{
    AssignmentEntry, AssignmentInfo, AssignmentRoleUpdate,
};
use crate::part::repo::oper::assignment::{
    CreateAssignment, DeleteAssignments, FindAssignmentInfo, GetAssignmentInfo,
    ListAssignmentInfos, ListAssignmentInfosExcluded, UpdateAssignmentRoles,
};
use crate::part_impl::repo::rdb_impl::entity::assignment::{
    AssignmentAspect, AssignmentRoleTimestamps, AssignmentRow,
    AssignmentRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_assignment::dsl::*;
use crate::part_impl::repo::rdb_impl::schema::t_chapter::{
    f_comic_id as chapter_comic_id, table as chapter_table,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, incl};
use crate::part_impl::shared::result::{diesel, expected};
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{BaseError, BaseResult, accept};
use crate::value::assignment::AssignmentInclOpt;

#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

mod list;

/// Converts a single `AssignmentRow` into an `AssignmentInfo`.
fn row_into_info(row: AssignmentRow) -> BaseResult<AssignmentInfo> {
    row.try_into()
}

/// Converts a vector of `AssignmentRow` values into a vector of `AssignmentInfo`.
fn rows_into_infos(
    rows: Vec<AssignmentRow>,
) -> BaseResult<Vec<AssignmentInfo>> {
    rows.into_iter().map(row_into_info).collect()
}

/// Queries a single assignment row by chapter ID and user ID, returning `None` if not found.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_by_chapter_id_and_user_id(
    conn: &mut RdbConn,
    chapter_id: &str,
    user_id: &str,
) -> BaseResult<Option<AssignmentInfo>> {
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

/// Queries one assignment for a user and comic, returning `None` if absent.
#[instrument(level = "info", err(Debug), skip_all)]
async fn find_info_by_user_id_and_comic_id(
    conn: &mut RdbConn,
    user_id: &str,
    comic_id: &str,
    incls: &[AssignmentInclOpt],
) -> BaseResult<Option<AssignmentInfo>> {
    //
    let row: Option<AssignmentRow> = t_assignment
        .inner_join(chapter_table)
        .filter(f_user_id.eq(user_id))
        .filter(chapter_comic_id.eq(comic_id))
        .select(AssignmentRow::as_select())
        .order_by((f_created_at.desc(), f_id.asc()))
        .get_result(conn)
        .await
        .optional()
        .map_err(diesel)?;

    let Some(row) = row else {
        return accept(None);
    };

    let mut assignment_info = row_into_info(row)?;

    incl::assignment::populate_assignment_incls(
        conn,
        std::slice::from_mut(&mut assignment_info),
        incls,
    )
    .await?;

    accept(Some(assignment_info))
}

/// Queries a single assignment row by ID and populates its includes.
#[instrument(level = "info", err(Debug), skip_all)]
async fn get_info_by_id(
    conn: &mut RdbConn,
    id: &str,
    incl_opt: &[AssignmentInclOpt],
) -> BaseResult<AssignmentInfo> {
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

    accept(info)
}

/// Queries all assignment rows for a chapter under `FOR UPDATE` lock.
#[instrument(level = "info", err(Debug), skip_all)]
async fn list_chapter_assignments_excluded(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseResult<Vec<AssignmentInfo>> {
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

/// Inserts a new assignment row from the given entry and returns the created info.
#[instrument(level = "info", err(Debug), skip_all)]
async fn create(
    conn: &mut RdbConn,
    model_entry: &AssignmentEntry,
) -> BaseResult<AssignmentInfo> {
    //
    let now = OffsetDateTime::now_utc();

    let entry = AssignmentRowEntry::from_model_entry(model_entry, now);

    let row: AssignmentRow = diesel::insert_into(t_assignment)
        .values(&entry)
        .returning(AssignmentRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    row_into_info(row)
}

/// Updates the role timestamps for an assignment row.
#[instrument(level = "info", err(Debug), skip_all)]
async fn put_roles(
    conn: &mut RdbConn,
    update: &AssignmentRoleUpdate,
) -> BaseResult<AssignmentInfo> {
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

/// Deletes a single assignment row by ID.
#[instrument(level = "info", err(Debug), skip_all)]
async fn delete(conn: &mut RdbConn, id: &str) -> BaseResult<()> {
    //
    diesel::delete(t_assignment.filter(f_id.eq(id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

/// Deletes all assignment rows for a given chapter ID.
#[instrument(level = "info", err(Debug), skip_all)]
async fn delete_by_chapter_id(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseResult<()> {
    //
    diesel::delete(t_assignment.filter(f_chapter_id.eq(chapter_id)))
        .execute(conn)
        .await
        .map_err(diesel)?;

    accept(())
}

impl Run<FindAssignmentInfo<'_, '_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &FindAssignmentInfo<'_, '_>,
    ) -> BaseResult<Option<AssignmentInfo>> {
        match oper {
            //
            FindAssignmentInfo::ChapterUser {
                chapter_id,
                user_id,
            } => submit_query!(
                self.core,
                get_info_by_chapter_id_and_user_id,
                chapter_id,
                user_id
            ),

            FindAssignmentInfo::UserComic {
                user_id,
                comic_id,
                incls,
            } => submit_query!(
                self.core,
                find_info_by_user_id_and_comic_id,
                user_id,
                comic_id,
                incls
            ),
        }
    }
}

impl Run<ListAssignmentInfos<'_, '_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListAssignmentInfos<'_, '_>,
    ) -> BaseResult<Vec<AssignmentInfo>> {
        submit_query!(self.core, list_infos, oper)
    }
}

impl Run<GetAssignmentInfo<'_, '_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetAssignmentInfo<'_, '_>,
    ) -> BaseResult<AssignmentInfo> {
        submit_query!(self.core, get_info_by_id, oper.id, oper.incls)
    }
}

impl Step<ListAssignmentInfos<'_, '_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListAssignmentInfos<'_, '_>,
    ) -> BaseResult<Vec<AssignmentInfo>> {
        list_infos(context.conn(), oper).await
    }
}

impl Step<FindAssignmentInfo<'_, '_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &FindAssignmentInfo<'_, '_>,
    ) -> BaseResult<Option<AssignmentInfo>> {
        match oper {
            //
            FindAssignmentInfo::ChapterUser {
                chapter_id,
                user_id,
            } => {
                get_info_by_chapter_id_and_user_id(
                    context.conn(),
                    chapter_id,
                    user_id,
                )
                .await
            }

            FindAssignmentInfo::UserComic {
                user_id,
                comic_id,
                incls,
            } => {
                find_info_by_user_id_and_comic_id(
                    context.conn(),
                    user_id,
                    comic_id,
                    incls,
                )
                .await
            }
        }
    }
}

impl Step<ListAssignmentInfosExcluded<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ListAssignmentInfosExcluded<'_>,
    ) -> BaseResult<Vec<AssignmentInfo>> {
        match oper {
            ListAssignmentInfosExcluded::Chapter { chapter_id } => {
                list_chapter_assignments_excluded(context.conn(), chapter_id)
                    .await
            }
        }
    }
}

impl Step<CreateAssignment<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateAssignment<'_>,
    ) -> BaseResult<AssignmentInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl Step<UpdateAssignmentRoles<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateAssignmentRoles<'_>,
    ) -> BaseResult<AssignmentInfo> {
        put_roles(context.conn(), oper.update).await
    }
}

impl Step<DeleteAssignments<'_>, RdbContext> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteAssignments<'_>,
    ) -> BaseResult<()> {
        match oper {
            //
            DeleteAssignments::Id { id } => delete(context.conn(), id).await,

            DeleteAssignments::Chapter { chapter_id } => {
                delete_by_chapter_id(context.conn(), chapter_id).await
            }
        }
    }
}
