//! In-memory comic archive repository operations for use-case tests.

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::comic_archive::{
    ComicArchiveChapterSnapshot, ComicArchivePageSnapshot,
    ComicArchiveSnapshot, ComicArchiveWrite,
};
use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, GetComicArchiveSnapshotExcluded,
    ListComicArchivePayloads,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, expected, unrecoverable,
};
use crate::result::{BaseError, BaseResult, accept};

impl Run<ListComicArchivePayloads<'_>> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListComicArchivePayloads<'_>,
    ) -> BaseResult<Vec<(time::OffsetDateTime, String)>> {
        //
        let state = self.state.lock().unwrap();

        let payloads = state
            .comic_archives
            .iter()
            .filter(|record| record.team_id == oper.team_id)
            .filter(|record| {
                oper.months.iter().any(|month| {
                    record.created_at >= month.start
                        && record.created_at < month.end
                })
            })
            .map(|record| (record.created_at, record.archived_payload.clone()))
            .collect();

        accept(payloads)
    }
}

/// Clone a fully assembled archive snapshot from locked mock state.
fn get_snapshot_excluded(
    context: &mut MockContext,
    source_comic_id: &str,
) -> BaseResult<ComicArchiveSnapshot> {
    //
    let comic_info = context
        .state
        .comics
        .iter()
        .find(|comic_info| comic_info.id == source_comic_id)
        .cloned()
        .ok_or_else(|| expected("error-comic-not-found"))?;

    let workset_info = context
        .state
        .worksets
        .iter()
        .find(|workset_info| workset_info.id == comic_info.workset_id)
        .cloned()
        .ok_or_else(|| expected("error-workset-not-found"))?;

    let chapter_snapshots = context
        .state
        .chapters
        .iter()
        .filter(|chapter_info| chapter_info.comic_id == comic_info.id)
        .cloned()
        .map(|chapter_info| {
            //
            let assignment_infos = context
                .state
                .assignments
                .iter()
                .filter(|assignment_info| {
                    assignment_info.chapter_id == chapter_info.id
                })
                .cloned()
                .map(|mut assignment_info| {
                    //
                    assignment_info.user = Some(
                        context
                            .state
                            .users
                            .iter()
                            .find(|user_info| {
                                user_info.id == assignment_info.user_id
                            })
                            .cloned()
                            .ok_or_else(|| expected("error-user-not-found"))?,
                    );

                    accept(assignment_info)
                })
                .collect::<BaseResult<Vec<_>>>()?;

            let page_snapshots = context
                .state
                .pages
                .iter()
                .filter(|page_info| page_info.chapter_id == chapter_info.id)
                .cloned()
                .map(|page_info| {
                    //
                    let unit_infos = context
                        .state
                        .units
                        .iter()
                        .filter(|unit_info| unit_info.page_id == page_info.id)
                        .cloned()
                        .collect();

                    ComicArchivePageSnapshot {
                        page_info,
                        unit_infos,
                    }
                })
                .collect();

            accept(ComicArchiveChapterSnapshot {
                chapter_info,
                assignment_infos,
                page_snapshots,
            })
        })
        .collect::<BaseResult<Vec<_>>>()?;

    accept(ComicArchiveSnapshot {
        comic_info,
        workset_info,
        chapter_snapshots,
    })
}

/// Persist one archive row and delete active records in the mock transaction state.
fn commit(
    context: &mut MockContext,
    comic_archive_write: &ComicArchiveWrite,
) -> BaseResult<()> {
    //
    if context.archive_commit_failure {
        return Err(unrecoverable(
            "[MockComicArchive::commit] injected archive commit failure",
        ));
    }

    context
        .state
        .comic_archives
        .push(comic_archive_write.record.clone());

    context
        .state
        .assignment_invitations
        .retain(|assignment_invitation_info| {
            !comic_archive_write
                .source_chapter_ids
                .contains(&assignment_invitation_info.chapter_id)
        });

    context.state.assignments.retain(|assignment_info| {
        !comic_archive_write
            .source_chapter_ids
            .contains(&assignment_info.chapter_id)
    });

    context.state.units.retain(|unit_info| {
        !comic_archive_write
            .source_page_ids
            .contains(&unit_info.page_id)
    });

    context.state.pages.retain(|page_info| {
        !comic_archive_write
            .source_chapter_ids
            .contains(&page_info.chapter_id)
    });

    context.state.chapters.retain(|chapter_info| {
        !comic_archive_write
            .source_chapter_ids
            .contains(&chapter_info.id)
    });

    context.state.comics.retain(|comic_info| {
        comic_info.id != comic_archive_write.source_comic_id
    });

    accept(())
}

impl<'a> Step<GetComicArchiveSnapshotExcluded<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetComicArchiveSnapshotExcluded<'a>,
    ) -> BaseResult<ComicArchiveSnapshot> {
        get_snapshot_excluded(context, oper.comic_id)
    }
}

impl<'a> Step<CommitComicArchive<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CommitComicArchive<'a>,
    ) -> BaseResult<()> {
        commit(context, oper.write)
    }
}
