//! Atomic review writes through typed Diesel expressions.

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Step};
use tracing::instrument;

use crate::part::nucl::ReptRead;
use crate::part::repo::oper::issue_edit::{
    RenameIssueVariants, ReplacePageIssues,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::issue::IssueItem;
use crate::part_impl::repo::rdb_impl::issue_edit::revision;
use crate::part_impl::repo::rdb_impl::schema::{t_issue, t_page_artwork};
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;
use crate::shared::result::diesel as map_diesel;

impl<L> Step<ReplacePageIssues<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Required transaction isolation.
    type Level = ReptRead;
    // Domain error returned by this operation.
    type Error = BaseError;

    // Executes the typed repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ReplacePageIssues<'_>,
    ) -> BaseRest<()> {
        //
        let conn = context.conn();

        diesel::delete(
            t_issue::table.filter(
                t_issue::f_page_artwork_id.eq(&oper.repl.page_artwork_id),
            ),
        )
        .execute(conn)
        .await
        .map_err(map_diesel)?;

        for entries in oper.repl.entries.chunks(2000) {
            //
            let items = entries
                .iter()
                .map(IssueItem::try_from)
                .collect::<BaseRest<Vec<_>>>()?;

            diesel::insert_into(t_issue::table)
                .values(&items)
                .execute(conn)
                .await
                .map_err(map_diesel)?;
        }

        diesel::update(t_page_artwork::table.find(&oper.repl.page_artwork_id))
            .set(
                t_page_artwork::f_issue_revision
                    .eq(t_page_artwork::f_issue_revision + 1),
            )
            .execute(conn)
            .await
            .map_err(map_diesel)?;

        revision::bump_chapter(conn, &oper.repl.chapter_id).await
    }
}

impl<L> Step<RenameIssueVariants<'_>, RdbContext<L>> for HybRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Required transaction isolation.
    type Level = ReptRead;
    // Domain error returned by this operation.
    type Error = BaseError;

    // Executes the typed repository operation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &RenameIssueVariants<'_>,
    ) -> BaseRest<Vec<String>> {
        //
        let conn = context.conn();

        let page_ids = t_page_artwork::table
            .filter(t_page_artwork::f_chapter_id.eq(&oper.rename.chapter_id))
            .select(t_page_artwork::f_id);

        let changed = t_issue::table
            .filter(t_issue::f_page_artwork_id.eq_any(page_ids))
            .filter(t_issue::f_variant.eq(&oper.rename.from));

        let ids = changed
            .select(t_issue::f_page_artwork_id)
            .distinct()
            .load::<String>(conn)
            .await
            .map_err(map_diesel)?;

        if ids.is_empty() {
            return accept!(ids);
        }

        diesel::update(
            t_issue::table
                .filter(t_issue::f_page_artwork_id.eq_any(&ids))
                .filter(t_issue::f_variant.eq(&oper.rename.from)),
        )
        .set(t_issue::f_variant.eq(&oper.rename.to))
        .execute(conn)
        .await
        .map_err(map_diesel)?;

        diesel::update(
            t_page_artwork::table.filter(t_page_artwork::f_id.eq_any(&ids)),
        )
        .set(
            t_page_artwork::f_issue_revision
                .eq(t_page_artwork::f_issue_revision + 1),
        )
        .execute(conn)
        .await
        .map_err(map_diesel)?;

        revision::bump_chapter(conn, &oper.rename.chapter_id).await?;

        accept!(ids)
    }
}
