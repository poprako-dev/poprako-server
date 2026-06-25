//! Mock implementations of `ComicRepo` and `ComicRepoTransactional` for in-memory testing.

use async_trait::async_trait;
use poprako_transactional::advance::Advance;

use crate::complex::comic::ComicComplex;
use crate::model::comic::{ComicCoverReservation, ComicInfo};
use crate::part::repo::Execute;
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::step::comic::{
    Create, Delete, GetInfoById, GetInfoExcluded, ListByWorksetId, ListByWorksetIdExcluded,
    MarkCompleted, MarkCoverUploaded, ReserveCover, UpdateInfo,
};
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
impl<'a> Execute<ListByWorksetId<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &ListByWorksetId<'a>) -> Result<Vec<ComicInfo>, Self::Error> {
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
impl<'a> Advance<ListByWorksetIdExcluded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListByWorksetIdExcluded<'a>,
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
        context.state.comics.remove(pos);
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
