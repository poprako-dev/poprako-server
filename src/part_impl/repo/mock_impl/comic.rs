use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::complex::comic::ComicComplex;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::UserInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::read::spec::comic::ComicListSpec;
use crate::model::write::comic::ComicCoverReservation;
use crate::part::repo::oper::comic::{
    AllocComicChapterIndex, CreateComic, DeleteComic, GetComicInfo,
    GetComicInfoExcluded, ListComicInfos, ListComicInfosExcluded,
    MarkComicCoverUploaded, ReserveComicCover, TouchComicLastActive,
    UpdateComic, UpdateComicChapterCount,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseRest, accept};
use crate::value::chapter::StageMask;
use crate::value::comic::ComicInclOpt;
use crate::value::incl::expand_incl_opts;
use crate::value::index::user_index_to_stored_index;

// Find and clone a workset from mock storage by id.
fn find_workset(state: &MockState, workset_id: &str) -> Option<WorksetInfo> {
    //
    state
        .worksets
        .iter()
        .find(|workset_info| workset_info.id == workset_id)
        .cloned()
}

// Resolve the team owner of a workset for relation enrichment.
fn find_team_for_workset(
    state: &MockState,
    workset: &WorksetInfo,
) -> Option<TeamInfo> {
    //
    state
        .teams
        .iter()
        .find(|team_info| team_info.id == workset.team_id)
        .cloned()
}

// Resolve a user from mock state for creator relation enrichment.
fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    //
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

// Populate workset field when workset include is requested.
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

// Populate team field when team include is requested.
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

// Populate creator/workset/team fields according to include options.
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

// Apply relation includes to a comic summary in a stable order.
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
        //
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

// Check title/index fuzzy condition for list filtering.
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

// Check whether a comic matches list scope constraints.
fn comic_matches_stages(
    state: &MockState,
    comic_info: &ComicInfo,
    stages: Option<StageMask>,
) -> bool {
    //
    match stages {
        //
        Some(stage_mask) => state
            .chapters
            .iter()
            .find(|chapter_info| {
                chapter_info.comic_id == comic_info.id && chapter_info.is_pinned
            })
            .map(|chapter_info| chapter_info.stages.matches_filter(stage_mask))
            .unwrap_or(false),

        None => true,
    }
}

// Validate optimistic fields and toggle comic cover uploaded flag.
fn mark_comic_cover_uploaded(
    state: &mut MockState,
    id: &str,
    cover_version: u32,
    cover_key: Option<&str>,
    cover_uploaded: bool,
) -> BaseRest<()> {
    //
    let comic = state
        .comics
        .iter_mut()
        .find(|comic| comic.id == id)
        .ok_or_else(|| expected("error-comic-not-found"))?;

    if comic.cover_version != Some(cover_version)
        || cover_key.is_some_and(|cover_key| {
            comic.cover_key.as_deref() != Some(cover_key)
        })
    {
        return Err(expected("error-stale-cover-upload"));
    }

    comic.is_cover_uploaded = Some(cover_uploaded);

    comic.updated_at = now();

    accept(())
}

// Load one comic and hydrate include fields.
fn get_comic_info(
    state: &MockState,
    id: &str,
    incls: &[ComicInclOpt],
) -> BaseRest<ComicInfo> {
    //
    let mut comic_info = state
        .comics
        .iter()
        .find(|comic_info| comic_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-comic-not-found"))?;

    apply_comic_incls(state, &mut comic_info, incls);

    accept(comic_info)
}

// Build filtered, sorted and paginated comic lists.
fn list_comic_infos(state: &MockState, spec: &ComicListSpec) -> Vec<ComicInfo> {
    //
    let mut comic_infos = state
        .comics
        .iter()
        .filter(|comic_info| comic_info.workset_id == spec.workset_id)
        .filter(|comic_info| {
            //
            spec.fuzzy_title
                .as_ref()
                .map(|keyword| comic_matches_fuzzy(comic_info, keyword))
                .unwrap_or(true)
        })
        .filter(|comic_info| {
            comic_matches_stages(state, comic_info, spec.stages)
        })
        .cloned()
        .collect::<Vec<_>>();

    comic_infos.sort_by(|left, right| {
        //
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
    // Use base error type for get-by-id read operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Load locked state and delegate to shared helper.
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
    // Use base error type for list operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Load locked state and execute listing helper.
    async fn run(
        &self,
        oper: &ListComicInfos<'a>,
    ) -> Result<Vec<ComicInfo>, Self::Error> {
        //
        let state = self.state.lock().unwrap();

        accept(list_comic_infos(&state, oper.spec))
    }
}

impl<'a> Run<UpdateComic<'a>> for Mock {
    // Use base error type for full-run metadata updates.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Apply mutable field updates and touch updated_at.
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

        accept(())
    }
}

impl<'a> Run<MarkComicCoverUploaded<'a>> for Mock {
    // Use base error type for cover upload mark operations.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Validate and apply cover upload transition under lock.
    async fn run(
        &self,
        oper: &MarkComicCoverUploaded<'a>,
    ) -> Result<(), Self::Error> {
        //
        let mut state = self.state.lock().unwrap();

        mark_comic_cover_uploaded(
            &mut state,
            oper.id,
            oper.cover_version,
            oper.cover_key,
            oper.cover_uploaded,
        )
    }
}

impl<'a> Step<CreateComic<'a>, MockContext> for Mock {
    // Use base errors for create step inside transaction context.
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
            created_at: time,
            updated_at: time,
        };

        context.state.comics.push(comic.clone());

        accept(comic)
    }
}

impl<'a, 'b> Step<GetComicInfo<'a, 'b>, MockContext> for Mock {
    // Use base errors for mocked transaction get.
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
            return Err(expected("error-invalid-image-extension"));
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
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Validate existence and compute next chapter index from current count.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &AllocComicChapterIndex<'a>,
    ) -> Result<i32, Self::Error> {
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
            .count() as i32;

        accept(index)
    }
}

impl<'a> Step<UpdateComicChapterCount<'a>, MockContext> for Mock {
    // Use base errors for chapter count updates in mock.
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

        comic.chapter_count += oper.delta;

        comic.updated_at = now();

        accept(())
    }
}

impl<'a> Step<TouchComicLastActive<'a>, MockContext> for Mock {
    // Use base errors for updating comic heartbeat timestamps.
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
