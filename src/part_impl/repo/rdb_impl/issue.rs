//! Typed storage for the Chapter's single current review.

#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
mod tests;

use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Run, Step};
use tracing::instrument;

use poprako_rdb_core::RdbConn;

use crate::model::read::proj::issue::IssueInfo;
use crate::model::write::issue::ChapterIssuesRepl;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::issue::{ListIssueInfos, ReplaceChapterIssues};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::issue::{
    IssueEntryRow, IssueRow,
};
use crate::part_impl::repo::rdb_impl::schema::{t_issue, t_page_artwork};
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::RdbContext;
use crate::shared::result::diesel as map_diesel;

// Delete all details for a Chapter using its Chapter identity.
#[instrument(level = "info", skip_all)]
async fn clear(conn: &mut RdbConn, chapter_id: &str) -> BaseRest<()> {
    //
    let ids = t_page_artwork::table
        .filter(t_page_artwork::f_chapter_id.eq(chapter_id))
        .select(t_page_artwork::f_id);

    diesel::delete(
        t_issue::table.filter(t_issue::f_page_artwork_id.eq_any(ids)),
    )
    .execute(conn)
    .await
    .map_err(map_diesel)?;

    accept(())
}

// Read ordered issues without exposing mutation operations per Page or issue.
#[instrument(level = "info", skip_all)]
async fn list_infos(
    conn: &mut RdbConn,
    chapter_id: &str,
) -> BaseRest<Vec<IssueInfo>> {
    //
    let rows = t_issue::table
        .inner_join(t_page_artwork::table)
        .filter(t_page_artwork::f_chapter_id.eq(chapter_id))
        .order((t_page_artwork::f_index.asc(), t_issue::f_index.asc()))
        .select(IssueRow::as_select())
        .load::<IssueRow>(conn)
        .await
        .map_err(map_diesel)?;

    rows.into_iter().map(TryInto::try_into).collect()
}

// Chunk the validated replacement below PostgreSQL's bind parameter limit.
#[instrument(level = "info", skip_all)]
async fn replace(
    conn: &mut RdbConn,
    repl: &ChapterIssuesRepl<'_>,
) -> BaseRest<()> {
    //
    clear(conn, repl.chapter_id).await?;

    for entries in repl.entries.chunks(2000) {
        //
        let rows = entries
            .iter()
            .map(IssueEntryRow::try_from)
            .collect::<BaseRest<Vec<_>>>()?;

        diesel::insert_into(t_issue::table)
            .values(&rows)
            .execute(conn)
            .await
            .map_err(map_diesel)?;
    }

    accept(())
}

impl Run<ListIssueInfos<'_>> for HybRepo {
    // Application error returned by this operation.
    type Error = BaseError;

    // Executes this repository operation.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &ListIssueInfos<'_>) -> BaseRest<Vec<IssueInfo>> {
        submit_query!(self.rdb_core, list_infos, oper.chapter_id)
    }
}

impl<L> Step<ReplaceChapterIssues<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Minimum transaction isolation required by this operation.
    type Level = ReptRead;

    // Application error returned by this operation.
    type Error = BaseError;

    // Executes this repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ReplaceChapterIssues<'_>,
    ) -> BaseRest<()> {
        replace(context.conn(), oper.repl).await
    }
}
