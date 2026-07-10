//! RDB-backed comment repository.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use poprako_transactional::advance::Advance;

use crate::model::comment::{CommentForm, CommentInfo, CommentListSpec};
use crate::part::repo::comment::{CommentRepo, CommentRepoTransactional};
use crate::part::repo::step::comment::{Create, ListInfos};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::rdb_impl::entity::comment::{
    CommentEntry, CommentRow,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, RdbRepoTransactional, incl};
use crate::part_impl::shared::result::diesel;
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{RegularError, RegularResult};

use crate::part_impl::repo::rdb_impl::schema::t_comment::dsl::*;

impl CommentRepo<RdbContext> for RdbRepo {}

impl CommentRepoTransactional<RdbContext> for RdbRepoTransactional {}

/// Query comment infos matching the given list spec, with optional includes.
async fn list_infos(
    conn: &mut RdbConn,
    spec: &CommentListSpec,
) -> RegularResult<Vec<CommentInfo>> {
    //
    let rows: Vec<CommentRow> = t_comment
        .filter(f_team_id.eq(spec.team_id.as_str()))
        .select(CommentRow::as_select())
        .order_by(f_created_at.desc())
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
        .load(conn)
        .await
        .map_err(diesel)?;

    let mut infos: Vec<CommentInfo> =
        rows.into_iter().map(Into::into).collect();

    incl::comment::populate_comment_incls(conn, &mut infos, &spec.incl_opt)
        .await?;

    Ok(infos)
}

/// Insert a new comment from the given form and return the created info.
async fn create(
    conn: &mut RdbConn,
    form: &CommentForm,
) -> RegularResult<CommentInfo> {
    //
    let entry = CommentEntry::from(form);

    let row: CommentRow = diesel::insert_into(t_comment)
        .values(&entry)
        .returning(CommentRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(row.into())
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> RegularResult<Vec<CommentInfo>> {
        submit_query!(self.core, list_infos, step.spec)
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, RdbContext> for RdbRepoTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Create<'a>,
    ) -> RegularResult<CommentInfo> {
        create(context.conn(), step.form).await
    }
}
#[cfg(all(test, feature = "repo"))]
mod tests;
