//! Handler for the "comic_archive" prom topic.
//!
//! Dispatches [`ComicTask`] variants to their concrete implementations.

use std::sync::Arc;

use tracing::{Level, instrument};

use poprako_transactional::drive::Drive;

use crate::complex::comic::ComicComplex;
use crate::part::image::ImagePool;
use crate::part::prom::Prom;
use crate::part::prom::task::ComicTask;
use crate::part::repo::assignment::{
    AssignmentRepo, AssignmentRepoTransactional,
};
use crate::part::repo::assignment_invitation::{
    AssignmentInvitationRepo, AssignmentInvitationRepoTransactional,
};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::unit::{UnitRepo, UnitRepoTransactional};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::part_impl::shared::RdbContext;
use crate::result::{RegularError, RegularResult};
use crate::util::DeriveTransactional;

/// Dispatch a [`ComicTask`] to its concrete handler.
pub(crate) async fn handle<D, R, P, I>(
    drive: &D,
    repo: &Arc<R>,
    prom: &P,
    _image_pool: &I,
    task: &ComicTask<'_>,
) -> RegularResult<()>
where
    D: Drive<RdbContext>,
    D::Error: Into<RegularError>,
    R: DeriveTransactional
        + ComicRepo<RdbContext>
        + WorksetRepo<RdbContext>
        + ChapterRepo<RdbContext>
        + PageRepo<RdbContext>
        + AssignmentRepo<RdbContext>
        + AssignmentInvitationRepo<RdbContext>
        + UnitRepo<RdbContext>
        + Send
        + Sync,
    <R as DeriveTransactional>::Transactional:
        ComicRepoTransactional<RdbContext>
            + WorksetRepoTransactional<RdbContext>
            + ChapterRepoTransactional<RdbContext>
            + PageRepoTransactional<RdbContext>
            + AssignmentRepoTransactional<RdbContext>
            + AssignmentInvitationRepoTransactional<RdbContext>
            + UnitRepoTransactional<RdbContext>
            + Send
            + Sync,
    P: Prom<RdbContext> + Send + Sync,
    I: ImagePool + Send + Sync,
{
    match task {
        ComicTask::Archive { comic_id } => {
            handle_archive(drive, repo, prom, comic_id).await
        }
    }
}

#[instrument(skip(drive, repo, prom), level = Level::DEBUG)]
async fn handle_archive<D, R, P>(
    drive: &D,
    repo: &Arc<R>,
    prom: &P,
    comic_id: &str,
) -> RegularResult<()>
where
    D: Drive<RdbContext>,
    D::Error: Into<RegularError>,
    R: DeriveTransactional
        + ComicRepo<RdbContext>
        + WorksetRepo<RdbContext>
        + ChapterRepo<RdbContext>
        + PageRepo<RdbContext>
        + AssignmentRepo<RdbContext>
        + AssignmentInvitationRepo<RdbContext>
        + UnitRepo<RdbContext>
        + Send
        + Sync,
    <R as DeriveTransactional>::Transactional:
        ComicRepoTransactional<RdbContext>
            + WorksetRepoTransactional<RdbContext>
            + ChapterRepoTransactional<RdbContext>
            + PageRepoTransactional<RdbContext>
            + AssignmentRepoTransactional<RdbContext>
            + AssignmentInvitationRepoTransactional<RdbContext>
            + UnitRepoTransactional<RdbContext>
            + Send
            + Sync,
    P: Prom<RdbContext> + Send + Sync,
{
    let comic_id = comic_id.to_string();
    let repo = Arc::clone(repo);

    drive
        .with_context(async move |context| {
            let transactional = repo.derive_transactional().await;

            ComicComplex::delete_cascade(
                &transactional,
                prom,
                context,
                &comic_id,
            )
            .await
        })
        .await
        .map_err(|e| e.into())
}
