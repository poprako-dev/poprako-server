//! RDB-backed comment repository.

/// Comment RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::Run;
use tracing::instrument;

use crate::model::read::proj::comment::CommentInfo;
use crate::model::read::spec::comment::CommentListSpec;
use crate::model::write::comment::CommentEntry;
use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::comment::{
    CommentEntryRow, CommentInfoRow,
};
use crate::part_impl::repo::rdb_impl::incl;
use crate::part_impl::repo::rdb_impl::schema::t_comment::dsl::{
    f_created_at, f_team_id, t_comment,
};
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::RdbConn;
use crate::shared::result::diesel;

// Query comment infos matching the given list spec, with optional includes.
#[instrument(level = "info", skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    spec: &CommentListSpec,
) -> BaseRest<Vec<CommentInfo>> {
    //
    let rows = t_comment
        .filter(f_team_id.eq(spec.team_id.as_str()))
        .select(CommentInfoRow::as_select())
        .order_by(f_created_at.desc())
        .offset(i64::from(spec.offset))
        .limit(i64::from(spec.limit))
        .load::<CommentInfoRow>(conn)
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
#[instrument(level = "info", skip_all)]
async fn create(
    conn: &mut RdbConn,
    entry: &CommentEntry,
) -> BaseRest<CommentInfo> {
    //
    let entry = CommentEntryRow::from(entry);

    let row = diesel::insert_into(t_comment)
        .values(&entry)
        .returning(CommentInfoRow::as_returning())
        .get_result::<CommentInfoRow>(conn)
        .await
        .map_err(diesel)?;

    accept(row.into())
}

impl Run<ListCommentInfos<'_>> for HybRepo {
    // Error type for the Run trait impl on comment list query.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Executes the comment list query with the given operation spec.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ListCommentInfos<'_>,
    ) -> BaseRest<Vec<CommentInfo>> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}

impl Run<CreateComment<'_>> for HybRepo {
    // Error type for the comment creation query.
    type Error = BaseError;

    // Creates a comment independently.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &CreateComment<'_>) -> BaseRest<CommentInfo> {
        submit_query!(self.core, create, oper.entry)
    }
}
