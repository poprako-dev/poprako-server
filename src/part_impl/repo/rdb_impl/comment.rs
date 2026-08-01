//! RDB-backed comment repository.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::comment::CommentInfo;
use crate::model::read::spec::comment::CommentListSpec;
use crate::model::write::comment::CommentEntry;
use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::part_impl::repo::rdb_impl::entity::comment::{
    CommentRow, CommentRowEntry,
};
use crate::part_impl::repo::rdb_impl::schema::t_comment::dsl::*;
use crate::part_impl::repo::rdb_impl::{RdbRepo, incl};
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::result::diesel;
use crate::shared::{RdbConn, RdbContext};

/// Comment RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

// Query comment infos matching the given list spec, with optional includes.
#[instrument(level = "info", err(Debug), skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &CommentListSpec,
) -> BaseRest<Vec<CommentInfo>> {
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

    let mut infos = rows
        .into_iter()
        .map(Into::into)
        .collect::<Vec<CommentInfo>>();

    incl::comment::populate_comment_incls(conn, &mut infos, &spec.incl_opt)
        .await?;

    accept(infos)
}

// Insert a new comment from the given entry and return the created info.
#[instrument(level = "info", err(Debug), skip_all)]
async fn create(
    conn: &mut RdbConn,
    entry: &CommentEntry,
) -> BaseRest<CommentInfo> {
    //
    let entry = CommentRowEntry::from(entry);

    let row: CommentRow = diesel::insert_into(t_comment)
        .values(&entry)
        .returning(CommentRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    accept(row.into())
}

impl Run<ListCommentInfos<'_>> for RdbRepo {
    // Error type for the Run trait impl on comment list query.
    type Error = BaseError;

    // Executes the comment list query with the given operation spec.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListCommentInfos<'_>,
    ) -> BaseRest<Vec<CommentInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl Step<CreateComment<'_>, RdbContext> for RdbRepo {
    // Error type for the Step trait impl on comment creation.
    type Error = BaseError;

    // Runs comment creation within an existing transaction.
    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateComment<'_>,
    ) -> BaseRest<CommentInfo> {
        create(context.conn(), oper.entry).await
    }
}
