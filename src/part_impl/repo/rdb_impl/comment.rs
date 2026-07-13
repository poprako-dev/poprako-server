//! RDB-backed comment repository.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use poprako_orchestra::{Run, Step};

use crate::part::repo::comment::CommentRepo;
use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::part_impl::repo::rdb_impl::entity::comment::{
    CommentRow, CommentRowEntry,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, incl};
use crate::part_impl::shared::result::diesel;
use crate::part_impl::shared::{RdbConn, RdbContext};
use crate::result::{RegularError, RegularResult};

use crate::model::comment::{CommentEntry,CommentInfo,CommentListSpec};
use crate::part_impl::repo::rdb_impl::schema::t_comment::dsl::*;

impl CommentRepo<RdbContext> for RdbRepo {}

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

/// Insert a new comment from the given entry and return the created info.
async fn create(
    conn: &mut RdbConn,
    entry: &CommentEntry,
) -> RegularResult<CommentInfo> {
    //
    let entry = CommentRowEntry::from(entry);

    let row: CommentRow = diesel::insert_into(t_comment)
        .values(&entry)
        .returning(CommentRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(row.into())
}

impl Run<ListCommentInfos<'_>> for RdbRepo {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &ListCommentInfos<'_>,
    ) -> RegularResult<Vec<CommentInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl Step<CreateComment<'_>, RdbContext> for RdbRepo {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateComment<'_>,
    ) -> RegularResult<CommentInfo> {
        create(context.conn(), oper.entry).await
    }
}

#[cfg(all(test, feature = "repo"))]
mod tests;
