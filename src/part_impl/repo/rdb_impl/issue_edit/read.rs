//! Consistent review queries through typed Diesel expressions.

use diesel::prelude::{
    ExpressionMethods as _, QueryDsl as _, SelectableHelper as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Step};
use tracing::instrument;

use crate::model::read::proj::issue::IssueInfo;
use crate::model::read::proj::issue_edit::{
    IssueEditReviewVer, IssueEditVariantCount,
};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::issue_edit::{
    GetIssuePageRevision, GetReviewVer, ListIssueInfosByIds,
    ListIssueVariantCounts, ListPageIssueInfos,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::issue::IssueRecord;
use crate::part_impl::repo::rdb_impl::schema::{
    t_chapter, t_issue, t_page_artwork,
};
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;
use crate::shared::result::diesel as map_diesel;

impl<L> Step<GetReviewVer<'_>, RdbContext<L>> for HybRepo
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
        oper: &GetReviewVer<'_>,
    ) -> BaseRest<IssueEditReviewVer> {
        //
        let (gen_no, revision) = t_chapter::table
            .find(oper.chapter_id)
            .select((
                t_chapter::f_review_generation,
                t_chapter::f_review_revision,
            ))
            .first::<(i64, i64)>(context.conn())
            .await
            .map_err(map_diesel)?;

        accept!(IssueEditReviewVer { gen_no, revision })
    }
}

impl<L> Step<GetIssuePageRevision<'_>, RdbContext<L>> for HybRepo
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
        oper: &GetIssuePageRevision<'_>,
    ) -> BaseRest<i64> {
        //
        t_page_artwork::table
            .find(oper.page_artwork_id)
            .select(t_page_artwork::f_issue_revision)
            .first(context.conn())
            .await
            .map_err(map_diesel)
    }
}

impl<L> Step<ListPageIssueInfos<'_>, RdbContext<L>> for HybRepo
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
        oper: &ListPageIssueInfos<'_>,
    ) -> BaseRest<Vec<IssueInfo>> {
        //
        let rows = t_issue::table
            .filter(t_issue::f_page_artwork_id.eq(oper.page_artwork_id))
            .order(t_issue::f_index.asc())
            .select(IssueRecord::as_select())
            .load::<IssueRecord>(context.conn())
            .await
            .map_err(map_diesel)?;

        rows.into_iter().map(TryInto::try_into).collect()
    }
}

impl<L> Step<ListIssueInfosByIds<'_>, RdbContext<L>> for HybRepo
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
        oper: &ListIssueInfosByIds<'_>,
    ) -> BaseRest<Vec<IssueInfo>> {
        //
        let rows = t_issue::table
            .filter(t_issue::f_id.eq_any(oper.ids))
            .select(IssueRecord::as_select())
            .load::<IssueRecord>(context.conn())
            .await
            .map_err(map_diesel)?;

        rows.into_iter().map(TryInto::try_into).collect()
    }
}

impl<L> Step<ListIssueVariantCounts<'_>, RdbContext<L>> for HybRepo
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
        oper: &ListIssueVariantCounts<'_>,
    ) -> BaseRest<Vec<IssueEditVariantCount>> {
        //
        let rows = t_issue::table
            .inner_join(t_page_artwork::table)
            .filter(t_page_artwork::f_chapter_id.eq(oper.chapter_id))
            .group_by(t_issue::f_variant)
            .order(t_issue::f_variant.asc())
            .select((t_issue::f_variant, diesel::dsl::count_star()))
            .load::<(String, i64)>(context.conn())
            .await
            .map_err(map_diesel)?;

        accept!(
            rows.into_iter()
                .map(|(variant, count)| IssueEditVariantCount {
                    variant,
                    count
                })
                .collect()
        )
    }
}
