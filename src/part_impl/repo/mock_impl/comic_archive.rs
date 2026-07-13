//! In-memory comic archive repository operations for use-case tests.

use poprako_orchestra::Step;

use crate::model::comic_archive::{
    ComicArchiveChapterSnapshot, ComicArchivePageSnapshot,
    ComicArchiveSnapshot, ComicArchiveWrite,
};
use crate::part::repo::comic_archive::ComicArchiveRepo;
use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, GetComicArchiveSnapshotExcluded,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, expected, unrecoverable,
};
use crate::result::{RegularError, RegularResult};

impl ComicArchiveRepo<MockContext> for Mock {}

/// Clone a fully assembled archive snapshot from locked mock state.
fn get_snapshot_excluded(
    context: &mut MockContext,
    source_comic_id: &str,
) -> RegularResult<ComicArchiveSnapshot> {
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

                    ComicArchivePageSnapshot {
                        page_info,
                        unit_infos,
                    }
                })
                .collect();

            Ok(ComicArchiveChapterSnapshot {
                chapter_info,
                assignment_infos,
                page_snapshots,
            })
        })
        .collect::<RegularResult<Vec<_>>>()?;

    Ok(ComicArchiveSnapshot {
        comic_info,
        workset_info,
        chapter_snapshots,
    })
}

/// Persist archive rows and delete active records in the mock transaction state.
fn commit(
    context: &mut MockContext,
    comic_archive_write: &ComicArchiveWrite,
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

impl<'a> Step<GetComicArchiveSnapshotExcluded<'a>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetComicArchiveSnapshotExcluded<'a>,
    ) -> RegularResult<ComicArchiveSnapshot> {
        get_snapshot_excluded(context, oper.comic_id)
    }
}

impl<'a> Step<CommitComicArchive<'a>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CommitComicArchive<'a>,
    ) -> RegularResult<()> {
        commit(context, oper.write)
    }
}
