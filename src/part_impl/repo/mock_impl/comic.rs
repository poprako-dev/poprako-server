//! Mock implementations of `ComicRepo` and `ComicRepoTransactional` for in-memory testing.

use async_trait::async_trait;

use poprako_transactional::advance::Advance;

use crate::complex::comic::ComicComplex;
use crate::model::comic::{ComicCoverReservation, ComicInfo, ComicListKind};
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::model::workset::WorksetInfo;
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::step::comic::{
    Create, Delete, GetInfoById, GetInfoExcluded, IncrChapterNextIndex,
    ListInfos, ListInfosExcluded, MarkCoverUploaded, ReserveCover,
    TouchLastActive, UpdateChapterCount, UpdateInfo,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, MockTransactional, expected, now,
};
use crate::result::{RegularError, RegularResult};
use crate::value::comic::ComicInclOpt;
use crate::value::incl::expand_incl_opts;
use crate::value::index::user_index_to_stored_index;

impl ComicRepo<MockContext> for Mock {}

impl ComicRepoTransactional<MockContext> for MockTransactional {}

fn find_workset(state: &MockState, workset_id: &str) -> Option<WorksetInfo> {
    state
        .worksets
        .iter()
        .find(|workset_info| workset_info.id == workset_id)
        .cloned()
}

fn find_team_for_workset(
    state: &MockState,
    workset: &WorksetInfo,
) -> Option<TeamInfo> {
    state
        .teams
        .iter()
        .find(|team_info| team_info.id == workset.team_id)
        .cloned()
}

fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

fn apply_workset_incl(
    state: &MockState,
    comic_info: &mut ComicInfo,
    include_workset: bool,
) {
    //
    comic_info.workset = None;

    if include_workset {
        comic_info.workset = find_workset(state, &comic_info.workset_id);
    }
}

fn apply_team_incl(
    state: &MockState,
    comic_info: &mut ComicInfo,
    include_team: bool,
) {
    //
    comic_info.team = None;

    if !include_team {
        return;
    }

    let Some(workset_info) = &comic_info.workset else {
        return;
    };

    comic_info.team = find_team_for_workset(state, workset_info);
}

fn apply_creator_incl(
    state: &MockState,
    comic_info: &mut ComicInfo,
    include_creator: bool,
) {
    //
    comic_info.creator = None;

    if include_creator {
        comic_info.creator = find_user(state, &comic_info.creator_id);
    }
}

fn apply_comic_incls(
    state: &MockState,
    comic_info: &mut ComicInfo,
    incl_opt: &[ComicInclOpt],
) {
    //
    comic_info.workset = None;

    comic_info.team = None;

    comic_info.creator = None;

    for incl_opt in expand_incl_opts(incl_opt) {
        match incl_opt {
            //
            ComicInclOpt::Workset => {
                apply_workset_incl(state, comic_info, true)
            }

            ComicInclOpt::WorksetTeam => {
                apply_team_incl(state, comic_info, true)
            }

            ComicInclOpt::Creator => {
                apply_creator_incl(state, comic_info, true)
            }
        }
    }
}

fn comic_matches_kind(
    state: &MockState,
    comic_info: &ComicInfo,
    kind: &ComicListKind,
) -> bool {
    match kind {
        //
        ComicListKind::All => true,

        ComicListKind::Stages(stage_mask) => state
            .chapters
            .iter()
            .find(|chapter_info| {
                chapter_info.comic_id == comic_info.id && chapter_info.is_pinned
            })
            .map(|chapter_info| chapter_info.stages.matches_filter(*stage_mask))
            .unwrap_or(false),
    }
}

fn comic_matches_fuzzy(comic_info: &ComicInfo, fuzzy_title: &str) -> bool {
    //
    let composed_title = ComicComplex::compose_title(
        comic_info.index,
        &comic_info.author,
        &comic_info.title,
    )
    .to_lowercase();

    let fuzzy_title = fuzzy_title.to_lowercase();

    if composed_title.contains(fuzzy_title.as_str()) {
        return true;
    }

    match fuzzy_title.trim().parse() {
        //
        Ok(index) => user_index_to_stored_index(index)
            .map(|index| comic_info.index == index)
            .unwrap_or(false),

        Err(_) => false,
    }
}

/// Updates a comic record to mark its cover as uploaded, verifying the cover version
/// to detect stale uploads.
fn mark_comic_cover_uploaded(
    state: &mut MockState,
    id: &str,
    cover_version: i64,
) -> RegularResult<()> {
    //
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
    type Error = RegularError;

    async fn execute(
        &self,
        step: &GetInfoById<'a>,
    ) -> Result<ComicInfo, Self::Error> {
        let state = self.state.lock().unwrap();

        let mut info = state
            .comics
            .iter()
            .find(|comic| comic.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-comic-not-found"))?;

        apply_comic_incls(&state, &mut info, step.incl_opt);

        Ok(info)
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfos<'a>,
    ) -> Result<Vec<ComicInfo>, Self::Error> {
        let state = self.state.lock().unwrap();
        let mut comics = state
            .comics
            .iter()
            .filter(|comic| comic.workset_id == step.spec.workset_id)
            .filter(|comic| {
                step.spec
                    .fuzzy_title
                    .as_ref()
                    .map(|kw| comic_matches_fuzzy(comic, kw))
                    .unwrap_or(true)
            })
            .filter(|comic| comic_matches_kind(&state, comic, &step.spec.kind))
            .cloned()
            .collect::<Vec<_>>();
        comics.sort_by(|left, right| {
            right
                .last_active_at
                .cmp(&left.last_active_at)
                .then_with(|| left.index.cmp(&right.index))
        });

        for comic in &mut comics {
            apply_comic_incls(&state, comic, &step.spec.incl_opt);
        }

        let offset = step.spec.offset as usize;
        let limit = step.spec.limit as usize;

        if offset >= comics.len() {
            return Ok(Vec::new());
        }

        let end = std::cmp::min(offset + limit, comics.len());
        Ok(comics[offset..end].to_vec())
    }
}

#[async_trait]
impl<'a> Execute<UpdateInfo<'a>> for Mock {
    type Error = RegularError;

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
    type Error = RegularError;

    async fn execute(
        &self,
        step: &MarkCoverUploaded<'a>,
    ) -> Result<(), Self::Error> {
        let mut state = self.state.lock().unwrap();
        mark_comic_cover_uploaded(&mut state, step.id, step.cover_version)
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

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
            cover_key: None,
            cover_uploaded: false,
            cover_version: 0,
            chapter_count: 0,
            chapter_next_index: 0,
            creator_id: step.form.creator_id.clone(),
            workset: None,
            team: None,
            creator: None,
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
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoById<'a>,
    ) -> Result<ComicInfo, Self::Error> {
        let mut info = context
            .state
            .comics
            .iter()
            .find(|comic| comic.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-comic-not-found"))?;

        apply_comic_incls(&context.state, &mut info, step.incl_opt);

        Ok(info)
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoExcluded<'a>,
    ) -> Result<ComicInfo, Self::Error> {
        let mut info = context
            .state
            .comics
            .iter()
            .find(|comic| comic.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-comic-not-found"))?;

        apply_comic_incls(&context.state, &mut info, step.incl_opt);

        Ok(info)
    }
}

#[async_trait]
impl<'a> Advance<ListInfosExcluded<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ListInfosExcluded<'a>,
    ) -> Result<Vec<ComicInfo>, Self::Error> {
        let mut comics = context
            .state
            .comics
            .iter()
            .filter(|comic| comic.workset_id == step.spec.workset_id)
            .filter(|comic| {
                step.spec
                    .fuzzy_title
                    .as_ref()
                    .map(|kw| comic_matches_fuzzy(comic, kw))
                    .unwrap_or(true)
            })
            .filter(|comic| {
                comic_matches_kind(&context.state, comic, &step.spec.kind)
            })
            .cloned()
            .collect::<Vec<_>>();
        comics.sort_by(|left, right| {
            right
                .last_active_at
                .cmp(&left.last_active_at)
                .then_with(|| left.index.cmp(&right.index))
        });

        for comic in &mut comics {
            apply_comic_incls(&context.state, comic, &step.spec.incl_opt);
        }

        let offset = step.spec.offset as usize;
        let limit = step.spec.limit as usize;

        if offset >= comics.len() {
            return Ok(Vec::new());
        }

        let end = std::cmp::min(offset + limit, comics.len());
        Ok(comics[offset..end].to_vec())
    }
}

#[async_trait]
impl<'a> Advance<ReserveCover<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

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
        let object_key = ComicComplex::gen_cover_key(
            step.id,
            cover_version,
            step.file_extension,
        );
        let prev_object_key = comic.cover_key.clone();
        comic.cover_key = Some(object_key.clone());
        comic.cover_uploaded = false;
        comic.cover_version = cover_version;
        comic.updated_at = now();
        Ok(ComicCoverReservation {
            object_key,
            prev_object_key,
            cover_version,
        })
    }
}

#[async_trait]
impl<'a> Advance<MarkCoverUploaded<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &MarkCoverUploaded<'a>,
    ) -> Result<(), Self::Error> {
        mark_comic_cover_uploaded(
            &mut context.state,
            step.id,
            step.cover_version,
        )
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

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
impl<'a> Advance<IncrChapterNextIndex<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

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
        let index = comic.chapter_next_index;

        comic.chapter_next_index += 1;
        comic.updated_at = now();

        Ok(index)
    }
}

#[async_trait]
impl<'a> Advance<UpdateChapterCount<'a>, MockContext> for MockTransactional {
    type Error = RegularError;

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
    type Error = RegularError;

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
