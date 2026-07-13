use poprako_orchestra::{Run, Step};

use crate::complex::comic::ComicComplex;
use crate::model::comic::{
    ComicCoverReservation, ComicInfo, ComicInfoListKind, ComicInfoListSpec,
};
use crate::model::team::TeamInfo;
use crate::model::user::UserInfo;
use crate::model::workset::WorksetInfo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::comic::{
    AllocateComicChapterIndex, CreateComic, DeleteComic, GetComicInfo,
    GetComicInfoExcluded, ListComicInfos, ListComicInfosExcluded,
    MarkComicCoverUploaded, ReserveComicCover, TouchComicLastActive,
    UpdateComic, UpdateComicChapterCount,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{RegularError, RegularResult};
use crate::value::comic::ComicInclOpt;
use crate::value::incl::expand_incl_opts;
use crate::value::index::user_index_to_stored_index;

impl ComicRepo<MockContext> for Mock {}

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
    kind: &ComicInfoListKind,
) -> bool {
    match kind {
        //
        ComicInfoListKind::All => true,

        ComicInfoListKind::Stages(stage_mask) => state
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
    cover_version: u32,
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

fn get_comic_info(
    state: &MockState,
    id: &str,
    incls: &[ComicInclOpt],
) -> RegularResult<ComicInfo> {
    //
    let mut comic_info = state
        .comics
        .iter()
        .find(|comic_info| comic_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-comic-not-found"))?;

    apply_comic_incls(state, &mut comic_info, incls);

    Ok(comic_info)
}

fn list_comic_infos(
    state: &MockState,
    spec: &ComicInfoListSpec,
) -> Vec<ComicInfo> {
    //
    let mut comic_infos = state
        .comics
        .iter()
        .filter(|comic_info| comic_info.workset_id == spec.workset_id)
        .filter(|comic_info| {
            spec.fuzzy_title
                .as_ref()
                .map(|keyword| comic_matches_fuzzy(comic_info, keyword))
                .unwrap_or(true)
        })
        .filter(|comic_info| comic_matches_kind(state, comic_info, &spec.kind))
        .cloned()
        .collect::<Vec<_>>();

    comic_infos.sort_by(|left, right| {
        right
            .last_active_at
            .cmp(&left.last_active_at)
            .then_with(|| left.index.cmp(&right.index))
    });

    for comic_info in &mut comic_infos {
        apply_comic_incls(state, comic_info, &spec.incl_opt);
    }

    let offset = spec.offset as usize;

    let limit = spec.limit as usize;

    match offset >= comic_infos.len() {
        //
        true => Vec::new(),

        false => {
            //
            let end = std::cmp::min(offset + limit, comic_infos.len());

            comic_infos[offset..end].to_vec()
        }
    }
}

impl<'a, 'b> Run<GetComicInfo<'a, 'b>> for Mock {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &GetComicInfo<'a, 'b>,
    ) -> Result<ComicInfo, Self::Error> {
        //
        let state = self.state.lock().unwrap();

        get_comic_info(&state, oper.id, oper.incls)
    }
}

impl<'a> Run<ListComicInfos<'a>> for Mock {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &ListComicInfos<'a>,
    ) -> Result<Vec<ComicInfo>, Self::Error> {
        //
        let state = self.state.lock().unwrap();

        Ok(list_comic_infos(&state, oper.spec))
    }
}

impl<'a> Run<UpdateComic<'a>> for Mock {
    type Error = RegularError;

    async fn run(&self, oper: &UpdateComic<'a>) -> Result<(), Self::Error> {
        //
        let mut state = self.state.lock().unwrap();

        let comic = state
            .comics
            .iter_mut()
            .find(|comic| comic.id == oper.update.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;

        comic.title = oper.update.title.clone();

        comic.author = oper.update.author.clone();

        comic.description = oper.update.description.clone();

        comic.updated_at = now();

        Ok(())
    }
}

impl<'a> Run<MarkComicCoverUploaded<'a>> for Mock {
    type Error = RegularError;

    async fn run(
        &self,
        oper: &MarkComicCoverUploaded<'a>,
    ) -> Result<(), Self::Error> {
        //
        let mut state = self.state.lock().unwrap();

        mark_comic_cover_uploaded(&mut state, oper.id, oper.cover_version)
    }
}

impl<'a> Step<CreateComic<'a>, MockContext> for Mock {
    type Error = RegularError;

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
            cover_uploaded: false,
            cover_version: 0,
            chapter_count: 0,
            chapter_next_index: 0,
            creator_id: oper.entry.creator_id.clone(),
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

impl<'a, 'b> Step<GetComicInfo<'a, 'b>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetComicInfo<'a, 'b>,
    ) -> Result<ComicInfo, Self::Error> {
        get_comic_info(&context.state, oper.id, oper.incls)
    }
}

impl<'a, 'b> Step<GetComicInfoExcluded<'a, 'b>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetComicInfoExcluded<'a, 'b>,
    ) -> Result<ComicInfo, Self::Error> {
        get_comic_info(&context.state, oper.id, oper.incls)
    }
}

impl<'a> Step<ListComicInfosExcluded<'a>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListComicInfosExcluded<'a>,
    ) -> Result<Vec<ComicInfo>, Self::Error> {
        Ok(list_comic_infos(&context.state, oper.spec))
    }
}

impl<'a> Step<ListComicInfos<'a>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListComicInfos<'a>,
    ) -> Result<Vec<ComicInfo>, Self::Error> {
        Ok(list_comic_infos(&context.state, oper.spec))
    }
}

impl<'a> Step<ReserveComicCover<'a>, MockContext> for Mock {
    type Error = RegularError;

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

        let cover_version = comic.cover_version + 1;

        let object_key = ComicComplex::gen_cover_key(
            oper.id,
            cover_version,
            oper.file_extension,
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

impl<'a> Step<MarkComicCoverUploaded<'a>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &MarkComicCoverUploaded<'a>,
    ) -> Result<(), Self::Error> {
        mark_comic_cover_uploaded(
            &mut context.state,
            oper.id,
            oper.cover_version,
        )
    }
}

impl<'a> Step<DeleteComic<'a>, MockContext> for Mock {
    type Error = RegularError;

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

impl<'a> Step<AllocateComicChapterIndex<'a>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &AllocateComicChapterIndex<'a>,
    ) -> Result<i32, Self::Error> {
        //
        let comic = context
            .state
            .comics
            .iter_mut()
            .find(|comic| comic.id == oper.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;

        let index = comic.chapter_next_index;

        comic.chapter_next_index += 1;

        comic.updated_at = now();

        Ok(index)
    }
}

impl<'a> Step<UpdateComicChapterCount<'a>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateComicChapterCount<'a>,
    ) -> Result<(), Self::Error> {
        //
        let comic = context
            .state
            .comics
            .iter_mut()
            .find(|comic| comic.id == oper.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;

        comic.chapter_count += oper.delta;

        comic.updated_at = now();

        Ok(())
    }
}

impl<'a> Step<TouchComicLastActive<'a>, MockContext> for Mock {
    type Error = RegularError;

    async fn step(
        &self,
        context: &mut MockContext,
        oper: &TouchComicLastActive<'a>,
    ) -> Result<(), Self::Error> {
        //
        let comic = context
            .state
            .comics
            .iter_mut()
            .find(|comic| comic.id == oper.id)
            .ok_or_else(|| expected("error-comic-not-found"))?;

        comic.last_active_at = now();

        comic.updated_at = now();

        Ok(())
    }
}
