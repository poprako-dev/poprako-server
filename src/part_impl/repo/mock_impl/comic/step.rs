use poprako_orchestra::Step;
use tracing::instrument;

use crate::complex::comic::ComicComplex;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::write::comic::ComicCoverReservation;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::comic::{
    AllocComicChapterIndex, CreateComic, DeleteComic, GetComicInfo,
    GetComicInfoExcluded, ListComicInfos, ListComicInfosExcluded,
    MarkComicCoverUploaded, ReserveComicCover, TouchComicLastActive,
    UpdateComicChapterCount,
};
use crate::part_impl::repo::mock_impl::comic::{
    get_comic_info, list_comic_infos, mark_comic_cover_uploaded,
};
use crate::part_impl::repo::mock_impl::nucl::apply_signed_delta;
use crate::part_impl::repo::mock_impl::{Mock, MockContext, expected, now};
use crate::result::{BaseError, accept};

impl<'a> Step<CreateComic<'a>, MockContext> for Mock {
    // Use base errors for create step inside transaction context.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Check id collision, then insert a new comic model and return snapshot.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateComic<'a>,
    ) -> Result<ComicInfo, Self::Error> {
        //
        if context
            .state
            .comics
            .iter()
            .any(|comic| comic.id == oper.entry.id)
        {
            return Err(expected("error-already-exists"));
        }

        let time = now();

        let comic = ComicInfo {
            id: oper.entry.id.clone(),
            workset_id: oper.entry.workset_id.clone(),
            index: oper.entry.index,
            title: oper.entry.title.clone(),
            author: oper.entry.author.clone(),
            description: oper.entry.description.clone(),
            cover_key: None,
            is_cover_uploaded: None,
            cover_version: None,
            cover_hash: None,
            cover_ext: None,
            chapter_count: 0,
            creator_id: oper.entry.creator_id.clone(),
            workset: None,
            team: None,
            creator: None,
            last_active_at: time,
            archived_at: None,
            created_at: time,
            updated_at: time,
        };

        context.state.comics.push(comic.clone());

        accept(comic)
    }
}

impl<'a, 'b> Step<GetComicInfo<'a, 'b>, MockContext> for Mock {
    // Use base errors for mocked transaction get.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Load one comic and resolve its requested includes.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetComicInfo<'a, 'b>,
    ) -> Result<ComicInfo, Self::Error> {
        get_comic_info(&context.state, oper.id, oper.incls)
    }
}

impl<'a, 'b> Step<GetComicInfoExcluded<'a, 'b>, MockContext> for Mock {
    // Use base errors for excluded projection get operation.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Reuse shared read helper, applying exclusion-aware include list.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetComicInfoExcluded<'a, 'b>,
    ) -> Result<ComicInfo, Self::Error> {
        get_comic_info(&context.state, oper.id, oper.incls)
    }
}

impl<'a> Step<ListComicInfosExcluded<'a>, MockContext> for Mock {
    // Use base errors for transaction list operation.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Return list built by shared helper for excluded projection.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListComicInfosExcluded<'a>,
    ) -> Result<Vec<ComicInfo>, Self::Error> {
        accept(list_comic_infos(&context.state, oper.spec))
    }
}

impl<'a> Step<ListComicInfos<'a>, MockContext> for Mock {
    // Use base errors for transaction list operation.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Return list using shared filtering/sorting/page helper.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListComicInfos<'a>,
    ) -> Result<Vec<ComicInfo>, Self::Error> {
        accept(list_comic_infos(&context.state, oper.spec))
    }
}

impl<'a> Step<ReserveComicCover<'a>, MockContext> for Mock {
    // Use base errors for cover reservation steps.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Reuse cover key/version state and return existing or new reservation.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ReserveComicCover<'a>,
    ) -> Result<ComicCoverReservation, Self::Error> {
        //
        let comic = context
            .state
            .comics
            .iter_mut()
            .find(|comic| comic.id == oper.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;

        let same_hash = comic.cover_key.is_some()
            && comic.cover_hash.as_ref() == Some(oper.image_hash);

        if same_hash && comic.cover_ext != Some(oper.image_ext) {
            return Err(expected("error-image-extension-mismatch"));
        }

        if same_hash {
            //
            let object_key = comic.cover_key.clone().ok_or_else(|| {
                //
                BaseError::Unrecoverable {
                    message: "[Mock::ReserveComicCover] cover key is missing"
                        .into(),
                }
            })?;

            return accept(ComicCoverReservation {
                object_key,
                prev_object_key: None,
                cover_version: comic.cover_version.ok_or_else(|| {
                    //
                    BaseError::Unrecoverable {
                        message:
                            "[Mock::ReserveComicCover] cover version is missing"
                                .into(),
                    }
                })?,
                is_upload_required: comic.is_cover_uploaded != Some(true),
            });
        }

        let cover_version =
            comic.cover_version.unwrap_or(0).checked_add(1).ok_or_else(
                || BaseError::Unrecoverable {
                    message: "[Mock::ReserveComicCover] cover version overflow"
                        .into(),
                },
            )?;

        let object_key = ComicComplex::gen_cover_key(
            oper.id,
            cover_version,
            oper.image_ext.suffix(),
        );

        let prev_object_key = comic.cover_key.clone();

        comic.cover_key = Some(object_key.clone());

        comic.is_cover_uploaded = Some(false);

        comic.cover_version = Some(cover_version);

        comic.cover_hash = Some(oper.image_hash.clone());

        comic.cover_ext = Some(oper.image_ext);

        comic.updated_at = now();

        accept(ComicCoverReservation {
            object_key,
            prev_object_key,
            cover_version,
            is_upload_required: true,
        })
    }
}

impl<'a> Step<MarkComicCoverUploaded<'a>, MockContext> for Mock {
    // Use base errors for mock cover upload updates.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply cover upload state changes using shared helper after lock resolution.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &MarkComicCoverUploaded<'a>,
    ) -> Result<(), Self::Error> {
        //
        mark_comic_cover_uploaded(
            &mut context.state,
            oper.id,
            oper.cover_version,
            oper.cover_key,
            oper.cover_uploaded,
        )
    }
}

impl<'a> Step<DeleteComic<'a>, MockContext> for Mock {
    // Use base errors for mocked deletion operations.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Remove comic and cascade related in-memory entities.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteComic<'a>,
    ) -> Result<(), Self::Error> {
        //
        let pos = context
            .state
            .comics
            .iter()
            .position(|comic| comic.id == oper.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;

        let deleted_comic_id = context.state.comics[pos].id.clone();

        let deleted_chapter_ids = context
            .state
            .chapters
            .iter()
            .filter(|chapter_info| chapter_info.comic_id == deleted_comic_id)
            .map(|chapter_info| chapter_info.id.clone())
            .collect::<Vec<_>>();

        context.state.comics.remove(pos);

        context
            .state
            .chapters
            .retain(|chapter_info| chapter_info.comic_id != deleted_comic_id);

        context.state.pages.retain(|page_info| {
            //
            !deleted_chapter_ids
                .iter()
                .any(|chapter_id| chapter_id == &page_info.chapter_id)
        });

        context.state.assignments.retain(|assignment_info| {
            //
            !deleted_chapter_ids
                .iter()
                .any(|chapter_id| chapter_id == &assignment_info.chapter_id)
        });

        accept(())
    }
}

impl<'a> Step<AllocComicChapterIndex<'a>, MockContext> for Mock {
    // Use base errors for chapter index allocation in mock.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Validate existence and compute next chapter index from current count.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &AllocComicChapterIndex<'a>,
    ) -> Result<usize, Self::Error> {
        //
        // Validate comic exists before computing chapter count.
        context
            .state
            .comics
            .iter()
            .find(|comic| comic.id == oper.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;

        let index = context
            .state
            .chapters
            .iter()
            .filter(|chapter_info| chapter_info.comic_id == oper.id)
            .count();

        accept(index)
    }
}

impl<'a> Step<UpdateComicChapterCount<'a>, MockContext> for Mock {
    // Use base errors for chapter count updates in mock.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Update chapter count with delta and refresh the timestamp.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateComicChapterCount<'a>,
    ) -> Result<(), Self::Error> {
        //
        // Locate comic row and apply chapter count delta.
        let comic = context
            .state
            .comics
            .iter_mut()
            .find(|comic| comic.id == oper.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;

        apply_signed_delta(&mut comic.chapter_count, oper.delta)?;

        comic.updated_at = now();

        accept(())
    }
}

impl<'a> Step<TouchComicLastActive<'a>, MockContext> for Mock {
    // Use base errors for updating comic heartbeat timestamps.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Refresh last-active and updated timestamps for heartbeat signals.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &TouchComicLastActive<'a>,
    ) -> Result<(), Self::Error> {
        //
        // Update both heartbeat and updated timestamps.
        let comic = context
            .state
            .comics
            .iter_mut()
            .find(|comic| comic.id == oper.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;

        comic.last_active_at = now();

        comic.updated_at = now();

        accept(())
    }
}
