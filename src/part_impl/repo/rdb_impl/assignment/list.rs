use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use tracing::instrument;

use poprako_rdb_core::RdbConn;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::spec::assignment::AssignmentListSpec;
use crate::part::repo::oper::assignment::ListAssignmentInfos;
use crate::part_impl::repo::rdb_impl::entity::assignment::AssignmentInfoRecord;
use crate::part_impl::repo::rdb_impl::incl;
use crate::part_impl::repo::rdb_impl::schema::t_assignment::dsl::{
    f_assigned_proofreader_at, f_assigned_publisher_at,
    f_assigned_raw_provider_at, f_assigned_redrawer_at, f_assigned_reviewer_at,
    f_assigned_translator_at, f_assigned_typesetter_at, f_chapter_id,
    f_created_at, f_id, f_user_id, t_assignment,
};
use crate::result::BaseRest;
use crate::shared::result::diesel;
use crate::value::role::RoleField;

/// Queries assignment infos selected by the repository operation.
#[instrument(level = "info", skip_all)]
pub async fn list_infos(
    conn: &mut RdbConn,
    oper: &ListAssignmentInfos<'_, '_>,
) -> BaseRest<Vec<AssignmentInfo>> {
    //
    // Build the query shape from the operation mode first so the same pipeline can be reused
    // for all list variants.
    let (role, incl_opt, page, mut query) = match oper {
        //
        ListAssignmentInfos::Spec { spec } => match spec {
            //
            AssignmentListSpec::Chapter {
                chapter_id,
                role,
                incl_opt,
                offset,
                limit,
            } => (
                *role,
                incl_opt.as_slice(),
                Some((*offset, *limit)),
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
                *role,
                incl_opt.as_slice(),
                Some((*offset, *limit)),
                t_assignment
                    .filter(f_user_id.eq(owner_id.as_str()))
                    .into_boxed(),
            ),
        },

        ListAssignmentInfos::Chapter {
            chapter_id,
            role,
            incls,
        } => (
            *role,
            *incls,
            None,
            t_assignment
                .filter(f_chapter_id.eq(*chapter_id))
                .into_boxed(),
        ),

        ListAssignmentInfos::Chapters { chapter_ids, incls } => (
            None,
            *incls,
            None,
            t_assignment
                .filter(f_chapter_id.eq_any(*chapter_ids))
                .into_boxed(),
        ),
    };

    // Apply a role filter only when the caller explicitly requests one.
    if let Some(role) = role {
        //
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

            _ => query.filter(f_id.eq_any(Vec::<String>::new())),
        };
    }

    let query = query
        .select(AssignmentInfoRecord::as_select())
        .order_by((f_created_at.desc(), f_id.asc()));

    // Pull one page when pagination is set; otherwise return the full list.
    let records = match page {
        //
        Some((offset, limit)) => {
            //
            query
                .offset(i64::from(offset))
                .limit(i64::from(limit.get()))
                .load::<AssignmentInfoRecord>(conn)
                .await
        }

        None => query.load::<AssignmentInfoRecord>(conn).await,
    }
    .map_err(diesel)?;

    // Convert DB records to domain values, then hydrate the configured eager relations.
    let mut infos = records_into_infos(records)?;

    incl::assignment::populate_assignment_incls(conn, &mut infos, incl_opt)
        .await?;

    accept!(infos)
}

// Map query records into public-facing assignment infos by converting each record and
// bubbling mapping errors immediately.
fn records_into_infos(
    records: Vec<AssignmentInfoRecord>,
) -> BaseRest<Vec<AssignmentInfo>> {
    records.into_iter().map(record_into_info).collect()
}

// Convert one persisted assignment record into the API-facing info DTO.
fn record_into_info(record: AssignmentInfoRecord) -> BaseRest<AssignmentInfo> {
    record.try_into()
}
