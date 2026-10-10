//! In-memory composite manifests with rollback through MockContext.

use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use crate::complex::page_artwork as page_artwork_complex;
use crate::model::read::proj::page_artwork::PageArtworkInfo;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::page_artwork::{
    GetPageArtworkInfo, ListPageArtworkInfos, ReplacePageArtworkManifest,
    UpdatePageArtworkInfo,
};
use crate::part_impl::repo::mock_impl::{Mock, MockContext, MockState};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

// Read one Chapter's manifest in its own order.
fn list_infos(state: &MockState, chapter_id: &str) -> Vec<PageArtworkInfo> {
    //
    let mut infos = state
        .page_artworks
        .iter()
        .filter(|info| info.chapter_id == chapter_id)
        .cloned()
        .collect::<Vec<_>>();

    infos.sort_by_key(|info| info.index);

    infos
}

impl Run<GetPageArtworkInfo<'_>> for Mock {
    // Application error returned by this operation.
    type Error = BaseError;

    // Executes an independent composite page read.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &GetPageArtworkInfo<'_>,
    ) -> BaseRest<PageArtworkInfo> {
        //
        self.state
            .lock()
            .unwrap()
            .page_artworks
            .iter()
            .find(|info| info.id == oper.id)
            .cloned()
            .ok_or_else(|| {
                //
                page_artwork_complex::error(
                    ExpectedVariant::Args,
                    "error-page-artwork-not-found",
                )
            })
    }
}

impl Run<ListPageArtworkInfos<'_>> for Mock {
    // Application error returned by this operation.
    type Error = BaseError;

    // Executes an independent composite page read.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ListPageArtworkInfos<'_>,
    ) -> BaseRest<Vec<PageArtworkInfo>> {
        accept(list_infos(&self.state.lock().unwrap(), oper.chapter_id))
    }
}

impl Step<ListPageArtworkInfos<'_>, MockContext> for Mock {
    // Minimum transaction isolation required by this operation.
    type Level = ReptRead;

    // Application error returned by this operation.
    type Error = BaseError;

    // Executes composite persistence in the caller transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListPageArtworkInfos<'_>,
    ) -> BaseRest<Vec<PageArtworkInfo>> {
        accept(list_infos(&context.state, oper.chapter_id))
    }
}

impl Step<ReplacePageArtworkManifest<'_>, MockContext> for Mock {
    // Minimum transaction isolation required by this operation.
    type Level = ReptRead;

    // Application error returned by this operation.
    type Error = BaseError;

    // Executes composite persistence in the caller transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ReplacePageArtworkManifest<'_>,
    ) -> BaseRest<()> {
        //
        let removed_ids = context
            .state
            .page_artworks
            .iter()
            .filter(|info| {
                //
                info.chapter_id == oper.chapter_id
                    && !oper.entries.iter().any(|entry| entry.id == info.id)
            })
            .map(|info| info.id.clone())
            .collect::<Vec<_>>();

        context
            .state
            .issues
            .retain(|issue| !removed_ids.contains(&issue.page_artwork_id));

        context
            .state
            .page_artworks
            .retain(|info| !removed_ids.contains(&info.id));

        for entry in oper.entries {
            //
            let now = OffsetDateTime::now_utc();

            if let Some(info) = context
                .state
                .page_artworks
                .iter_mut()
                .find(|info| info.id == entry.id)
            {
                info.index = entry.index;

                info.raw_ident.clone_from(&entry.raw_ident);

                info.updated_at = now;

                continue;
            }

            context.state.page_artworks.push(PageArtworkInfo {
                id: entry.id.clone(),
                chapter_id: entry.chapter_id.clone(),
                index: entry.index,
                raw_ident: entry.raw_ident.clone(),
                created_at: now,
                updated_at: now,
            });
        }

        accept(())
    }
}

impl Step<UpdatePageArtworkInfo<'_>, MockContext> for Mock {
    // Minimum transaction isolation required by this operation.
    type Level = ReptRead;

    // Application error returned by this operation.
    type Error = BaseError;

    // Updates only the requested composite page metadata.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdatePageArtworkInfo<'_>,
    ) -> BaseRest<()> {
        //
        let page_artwork_info = context
            .state
            .page_artworks
            .iter_mut()
            .find(|info| info.id == oper.update.id)
            .ok_or_else(|| {
                //
                page_artwork_complex::error(
                    ExpectedVariant::Args,
                    "error-page-artwork-not-found",
                )
            })?;

        page_artwork_info
            .raw_ident
            .clone_from(&oper.update.raw_ident);

        page_artwork_info.updated_at = OffsetDateTime::now_utc();

        accept(())
    }
}
