//! Transactional review editing in the in-memory adapter.

use std::collections::BTreeMap;

use poprako_orchestra::Step;

use crate::model::read::proj::issue::IssueInfo;
use crate::model::read::proj::issue_edit::{
    IssueEditReviewVer, IssueEditVariantCount,
};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::issue_edit::{
    GetIssuePageRevision, GetReviewVer, ListIssueInfosByIds,
    ListIssueVariantCounts, ListPageIssueInfos, RenameIssueVariants,
    ReplacePageIssues,
};
use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::result::{BaseError, BaseRest};

/// Advances chapter context without changing the import generation.
pub fn bump_chapter(context: &mut MockContext, chapter_id: &str) {
    //
    context
        .state
        .review_versions
        .entry(chapter_id.to_owned())
        .or_default()
        .revision += 1;
}

impl Step<GetReviewVer<'_>, MockContext> for Mock {
    // Required transaction isolation.
    type Level = ReptRead;
    // Domain error returned by this operation.
    type Error = BaseError;

    // Executes the typed repository operation.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetReviewVer<'_>,
    ) -> BaseRest<IssueEditReviewVer> {
        //
        accept!(
            context
                .state
                .review_versions
                .get(oper.chapter_id)
                .copied()
                .unwrap_or_default()
        )
    }
}

impl Step<GetIssuePageRevision<'_>, MockContext> for Mock {
    // Required transaction isolation.
    type Level = ReptRead;
    // Domain error returned by this operation.
    type Error = BaseError;

    // Executes the typed repository operation.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetIssuePageRevision<'_>,
    ) -> BaseRest<i64> {
        //
        if !context
            .state
            .page_artworks
            .iter()
            .any(|page| page.id == oper.page_artwork_id)
        {
            return Err(crate::complex::issue::error(
                crate::result::ExpectedVariant::Args,
                "error-page-artwork-not-found",
            ));
        }

        accept!(
            context
                .state
                .issue_revisions
                .get(oper.page_artwork_id)
                .copied()
                .unwrap_or_default()
        )
    }
}

impl Step<ListPageIssueInfos<'_>, MockContext> for Mock {
    // Required transaction isolation.
    type Level = ReptRead;
    // Domain error returned by this operation.
    type Error = BaseError;

    // Executes the typed repository operation.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListPageIssueInfos<'_>,
    ) -> BaseRest<Vec<IssueInfo>> {
        //
        let mut issues = context
            .state
            .issues
            .iter()
            .filter(|issue| issue.page_artwork_id == oper.page_artwork_id)
            .cloned()
            .collect::<Vec<_>>();

        issues.sort_by_key(|issue| issue.index);

        accept!(issues)
    }
}

impl Step<ListIssueInfosByIds<'_>, MockContext> for Mock {
    // Required transaction isolation.
    type Level = ReptRead;
    // Domain error returned by this operation.
    type Error = BaseError;

    // Executes the typed repository operation.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListIssueInfosByIds<'_>,
    ) -> BaseRest<Vec<IssueInfo>> {
        //
        accept!(
            context
                .state
                .issues
                .iter()
                .filter(|issue| oper.ids.contains(&issue.id.as_str()))
                .cloned()
                .collect()
        )
    }
}

impl Step<ListIssueVariantCounts<'_>, MockContext> for Mock {
    // Required transaction isolation.
    type Level = ReptRead;
    // Domain error returned by this operation.
    type Error = BaseError;

    // Executes the typed repository operation.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListIssueVariantCounts<'_>,
    ) -> BaseRest<Vec<IssueEditVariantCount>> {
        //
        let pages = context
            .state
            .page_artworks
            .iter()
            .filter(|page| page.chapter_id == oper.chapter_id)
            .map(|page| page.id.as_str())
            .collect::<Vec<_>>();

        let mut counts = BTreeMap::<String, i64>::new();

        for issue in context
            .state
            .issues
            .iter()
            .filter(|issue| pages.contains(&issue.page_artwork_id.as_str()))
        {
            *counts.entry(issue.variant.clone()).or_default() += 1;
        }

        accept!(
            counts
                .into_iter()
                .map(|(variant, count)| IssueEditVariantCount {
                    variant,
                    count
                })
                .collect()
        )
    }
}

impl Step<ReplacePageIssues<'_>, MockContext> for Mock {
    // Required transaction isolation.
    type Level = ReptRead;
    // Domain error returned by this operation.
    type Error = BaseError;

    // Executes the typed repository operation.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ReplacePageIssues<'_>,
    ) -> BaseRest<()> {
        //
        context
            .state
            .issues
            .retain(|issue| issue.page_artwork_id != oper.repl.page_artwork_id);

        context
            .state
            .issues
            .extend(oper.repl.entries.iter().map(|entry| IssueInfo {
                id: entry.id.clone(),
                page_artwork_id: entry.page_artwork_id.clone(),
                index: entry.index,
                variant: entry.trimmed_variant().to_owned(),
                rect: entry.rect,
                note: entry.note.clone(),
            }));

        *context
            .state
            .issue_revisions
            .entry(oper.repl.page_artwork_id.clone())
            .or_default() += 1;

        bump_chapter(context, &oper.repl.chapter_id);

        accept!(())
    }
}

impl Step<RenameIssueVariants<'_>, MockContext> for Mock {
    // Required transaction isolation.
    type Level = ReptRead;
    // Domain error returned by this operation.
    type Error = BaseError;

    // Executes the typed repository operation.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &RenameIssueVariants<'_>,
    ) -> BaseRest<Vec<String>> {
        //
        let pages = context
            .state
            .page_artworks
            .iter()
            .filter(|page| page.chapter_id == oper.rename.chapter_id)
            .map(|page| page.id.as_str())
            .collect::<Vec<_>>();

        let mut changed = std::collections::BTreeSet::new();

        for issue in &mut context.state.issues {
            //
            if pages.contains(&issue.page_artwork_id.as_str())
                && issue.variant == oper.rename.from
            {
                issue.variant.clone_from(&oper.rename.to);

                changed.insert(issue.page_artwork_id.clone());
            }
        }

        for id in &changed {
            *context.state.issue_revisions.entry(id.clone()).or_default() += 1;
        }

        if !changed.is_empty() {
            bump_chapter(context, &oper.rename.chapter_id);
        }

        accept!(changed.into_iter().collect())
    }
}
