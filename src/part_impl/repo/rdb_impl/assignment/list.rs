use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tracing::instrument;

use crate::model::assignment::AssignmentInfo;
use crate::part_impl::repo::rdb_impl::entity::assignment::AssignmentRow;
use crate::part_impl::repo::rdb_impl::incl;
use crate::part_impl::repo::rdb_impl::schema::t_assignment::dsl::*;
use crate::part_impl::shared::RdbConn;
use crate::part_impl::shared::result::diesel;
use crate::result::{BaseResult, accept};
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::RoleField;

fn row_into_info(row: AssignmentRow) -> BaseResult<AssignmentInfo> {
    row.try_into()
}

fn rows_into_infos(
    rows: Vec<AssignmentRow>,
) -> BaseResult<Vec<AssignmentInfo>> {
    rows.into_iter().map(row_into_info).collect()
}

/// Queries all assignment rows for a given chapter, optionally filtered by role.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_all_infos_by_chapter(
    conn: &mut RdbConn,
    chapter_id: &str,
    role: Option<RoleField>,
    incl_opt: &[AssignmentInclOpt],
) -> BaseResult<Vec<AssignmentInfo>> {
    //
    let mut query = t_assignment
        .filter(f_chapter_id.eq(chapter_id))
        .into_boxed();

    if let Some(role) = role {
        query = match role {
            //
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

    accept(infos)
}

/// Queries all assignment rows for the given chapters.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_all_infos_by_chapters(
    conn: &mut RdbConn,
    chapter_ids: &[String],
    incl_opt: &[AssignmentInclOpt],
) -> BaseResult<Vec<AssignmentInfo>> {
    //
    let rows: Vec<AssignmentRow> = t_assignment
        .filter(f_chapter_id.eq_any(chapter_ids))
        .select(AssignmentRow::as_select())
        .order_by((f_created_at.desc(), f_id.asc()))
        .load(conn)
        .await
        .map_err(diesel)?;

    let mut infos = rows_into_infos(rows)?;

    incl::assignment::populate_assignment_incls(conn, &mut infos, incl_opt)
        .await?;

    accept(infos)
}
