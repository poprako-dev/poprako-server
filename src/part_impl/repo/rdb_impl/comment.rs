//! RDB-backed comment repository.

/// Comment RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
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
use crate::part_impl::repo::rdb_impl::schema::t_comment::dsl::*;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::result::diesel;
use crate::shared::{RdbConn, RdbContext};

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
        .offset(spec.offset as i64)
        .limit(spec.limit as i64)
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

impl<L> Step<CreateComment<'_>, RdbContext<L>> for HybRepo
where
    L: poprako_orchestra::Level + Send,
    L: poprako_orchestra::AtLeast<crate::part::nucl::RepeatableRead>,
{
    // Error type for the Step trait impl on comment creation.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Runs comment creation within an existing transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CreateComment<'_>,
    ) -> BaseRest<CommentInfo> {
        create(context.conn(), oper.entry).await
    }
}
