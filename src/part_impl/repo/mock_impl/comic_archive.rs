//! In-memory comic archive repository operations for use-case tests.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::model::comic_archive_model;
use crate::part::repo::comic_archive::ComicArchiveRepoTransactional;
use crate::part::repo::step::comic_archive::{Commit, LockSnapshot};
use crate::part_impl::repo::mock_impl::{
    MockContext, MockTransactional, expected, unrecoverable,
};
use crate::result::{RegularError, RegularResult};

impl ComicArchiveRepoTransactional<MockContext> for MockTransactional {}

/// Clone a fully assembled archive snapshot from locked mock state.
fn lock_snapshot(
    context: &mut MockContext,
    source_comic_id: &str,
) -> RegularResult<comic_archive_model::Snapshot> {
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

                    Ok(assignment_info)
                })
                .collect::<RegularResult<Vec<_>>>()?;

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

                    comic_archive_model::PageSnapshot {
                        page_info,
                        unit_infos,
                    }
                })
                .collect();

            Ok(comic_archive_model::ChapterSnapshot {
                chapter_info,
                assignment_infos,
                page_snapshots,
            })
        })
        .collect::<RegularResult<Vec<_>>>()?;

    Ok(comic_archive_model::Snapshot {
        comic_info,
        workset_info,
        chapter_snapshots,
    })
}

/// Persist archive rows and delete active records in the mock transaction state.
fn commit(
    context: &mut MockContext,
    comic_archive_write: &comic_archive_model::Write,
) -> RegularResult<()> {
    //
    if context.archive_commit_failure {
        return Err(unrecoverable(
            "[MockComicArchive::commit] injected archive commit failure",
        ));
    }

    context
        .state
        .archived_comics
        .push(comic_archive_write.comic_record.clone());

    context
        .state
        .archived_chapters
        .extend(comic_archive_write.chapter_records.iter().cloned());

    context
        .state
        .archived_translations
        .extend(comic_archive_write.translation_records.iter().cloned());

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

    Ok(())
}

#[async_trait]
impl<'a> Advance<LockSnapshot<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &LockSnapshot<'a>,
    ) -> RegularResult<comic_archive_model::Snapshot> {
        lock_snapshot(context, step.comic_id)
    }
}

#[async_trait]
impl<'a> Advance<Commit<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Commit<'a>,
    ) -> RegularResult<()> {
        commit(context, step.comic_archive_write)
    }
}
