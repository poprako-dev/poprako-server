//! Mock implementations of `ComicRepo` and `ComicRepoTransactional` for in-memory testing.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::complex::comic::ComicComplex;
use crate::model::comic::{ComicCoverReservation, ComicInfo};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::step::comic::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrChapterNextIndex, ListInfosByWorksetId,
    ListInfosByWorksetIdExcluded, MarkCompleted, MarkCoverUploaded, ReserveCover, TouchLastActive,
    UpdateChapterCount, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo_mock::{Mock, MockContext, MockState, MockTransactional, expected, now};
use crate::result::RootError;

impl ComicRepo<MockContext> for Mock {}

impl ComicRepoTransactional<MockContext> for MockTransactional {}

/// Updates a comic record to mark its cover as uploaded, verifying the cover version
/// to detect stale uploads.
fn mark_comic_cover_uploaded(
    state: &mut MockState,
    id: &str,
    cover_version: i64,
) -> Result<(), RootError> {
    let comic = state
        .comics
        .iter_mut()
        .find(|comic| comic.id == id)
        .ok_or_else(|| expected("error-comic-not-found"))?;
    if comic.cover_version != cover_version {
        return Err(expected("error-stale-cover-upload"));
    }
    comic.cover_uploaded = true;
    comic.updated_at = now();
    Ok(())
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &GetInfoById<'a>) -> Result<ComicInfo, Self::Error> {
        let state = self.state.lock().unwrap();
        state
            .comics
            .iter()
            .find(|comic| comic.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-comic-not-found"))
    }
}

#[async_trait]
impl<'a> Execute<ListInfosByWorksetId<'a>> for Mock {
    type Error = RootError;

    async fn execute(
        &self,
        step: &ListInfosByWorksetId<'a>,
    ) -> Result<Vec<ComicInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        let mut comics = state
            .comics
            .iter()
            .filter(|comic| comic.workset_id == step.workset_id)
            .cloned()
            .collect::<Vec<_>>();
        comics.sort_by(|left, right| left.index.cmp(&right.index));
        Ok(comics)
    }
}

#[async_trait]
impl<'a> Execute<UpdateInfo<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &UpdateInfo<'a>) -> Result<(), Self::Error> {
        let mut state = self.state.lock().unwrap();
        let comic = state
            .comics
            .iter_mut()
            .find(|comic| comic.id == step.update.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;
        comic.title = step.update.title.clone();
        comic.author = step.update.author.clone();
        comic.description = step.update.description.clone();
        comic.updated_at = now();
        Ok(())
    }
}

#[async_trait]
impl<'a> Execute<MarkCoverUploaded<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &MarkCoverUploaded<'a>) -> Result<(), Self::Error> {
        let mut state = self.state.lock().unwrap();
        mark_comic_cover_uploaded(&mut state, step.id, step.cover_version)
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Create<'a>,
    ) -> Result<ComicInfo, Self::Error> {
        if context
            .state
            .comics
            .iter()
            .any(|comic| comic.id == step.form.id)
        {
            return Err(expected("error-already-exists"));
        }

        let time = now();
        let comic = ComicInfo {
            id: step.form.id.clone(),
            workset_id: step.form.workset_id.clone(),
            index: step.form.index,
            title: step.form.title.clone(),
            author: step.form.author.clone(),
            description: step.form.description.clone(),
            is_completed: false,
            cover_key: None,
            cover_uploaded: false,
            cover_version: 0,
            chapter_count: 0,
            chapter_next_index: 0,
            creator_id: step.form.creator_id.clone(),
            last_active_at: time,
            created_at: time,
            updated_at: time,
        };
        context.state.comics.push(comic.clone());
        Ok(comic)
    }
}

#[async_trait]
impl<'a> Advance<GetInfoById<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoById<'a>,
    ) -> Result<ComicInfo, Self::Error> {
        context
            .state
            .comics
            .iter()
            .find(|comic| comic.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-comic-not-found"))
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoExcluded<'a>,
    ) -> Result<ComicInfo, Self::Error> {
        context
            .state
            .comics
            .iter()
            .find(|comic| comic.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-comic-not-found"))
    }
}

#[async_trait]
impl<'a> Advance<ListInfosByWorksetIdExcluded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListInfosByWorksetIdExcluded<'a>,
    ) -> Result<Vec<ComicInfo>, Self::Error> {
        let mut comics = context
            .state
            .comics
            .iter()
            .filter(|comic| comic.workset_id == step.workset_id)
            .cloned()
            .collect::<Vec<_>>();
        comics.sort_by(|left, right| left.index.cmp(&right.index));
        Ok(comics)
    }
}

#[async_trait]
impl<'a> Advance<ReserveCover<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ReserveCover<'a>,
    ) -> Result<ComicCoverReservation, Self::Error> {
        let comic = context
            .state
            .comics
            .iter_mut()
            .find(|comic| comic.id == step.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;
        let cover_version = comic.cover_version + 1;
        let object_key = ComicComplex::gen_cover_key(step.id, cover_version, step.file_extension);
        let previous_object_key = comic.cover_key.clone();
        comic.cover_key = Some(object_key.clone());
        comic.cover_uploaded = false;
        comic.cover_version = cover_version;
        comic.updated_at = now();
        Ok(ComicCoverReservation {
            object_key,
            previous_object_key,
            cover_version,
        })
    }
}

#[async_trait]
impl<'a> Advance<MarkCoverUploaded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &MarkCoverUploaded<'a>,
    ) -> Result<(), Self::Error> {
        mark_comic_cover_uploaded(&mut context.state, step.id, step.cover_version)
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Delete<'a>,
    ) -> Result<(), Self::Error> {
        let pos = context
            .state
            .comics
            .iter()
            .position(|comic| comic.id == step.id)
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
            !deleted_chapter_ids
                .iter()
                .any(|chapter_id| chapter_id == &page_info.chapter_id)
        });
        context.state.assignments.retain(|assignment_info| {
            !deleted_chapter_ids
                .iter()
                .any(|chapter_id| chapter_id == &assignment_info.chapter_id)
        });
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<MarkCompleted<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &MarkCompleted<'a>,
    ) -> Result<(), Self::Error> {
        let comic = context
            .state
            .comics
            .iter_mut()
            .find(|comic| comic.id == step.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;
        comic.is_completed = step.is_completed;
        comic.updated_at = now();
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<IncrChapterNextIndex<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &IncrChapterNextIndex<'a>,
    ) -> Result<i32, Self::Error> {
        let comic = context
            .state
            .comics
            .iter_mut()
            .find(|comic| comic.id == step.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;
        comic.chapter_next_index += 1;
        comic.updated_at = now();
        Ok(comic.chapter_next_index)
    }
}

#[async_trait]
impl<'a> Advance<UpdateChapterCount<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &UpdateChapterCount<'a>,
    ) -> Result<(), Self::Error> {
        let comic = context
            .state
            .comics
            .iter_mut()
            .find(|comic| comic.id == step.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;
        comic.chapter_count += step.delta;
        comic.updated_at = now();
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<TouchLastActive<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &TouchLastActive<'a>,
    ) -> Result<(), Self::Error> {
        let comic = context
            .state
            .comics
            .iter_mut()
            .find(|comic| comic.id == step.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;
        comic.last_active_at = now();
        comic.updated_at = now();
        Ok(())
    }
}
