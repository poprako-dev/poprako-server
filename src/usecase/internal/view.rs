//! Complete request-scoped response hydration and presentation.

// Object discovery, URL loading, and nested rendering implementation.
mod snapshot;

/// Shared object URL loading for response presentation.
pub mod obj_urls;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use poprako_orchestra::{Context, Run};

use poprako_obj_dept::ObjDeptView;

use crate::data::val::comic_list::ListComicInfosVal;
use crate::data::view::assignment::AssignmentInfoView;
use crate::data::view::chapter::ChapterInfoView;
use crate::data::view::comic::ComicInfoView;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::part::repo::oper::chapter::ListPinnedChapterInfos;
use crate::part::repo::oper::page::ListFirstPageInfos;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::internal::page::PinnedChapterSnapshot;
use crate::usecase::internal::view::snapshot::{ObjViewIds, ObjViewSnapshot};

/// Loaded Comic list models owned by one complete presentation request.
pub struct ComicListData {
    /// Comics in the requested response order.
    comics: Vec<ComicInfo>,

    /// Loaded pinned chapters and their known-present or absent positions.
    pinned_chapters: Option<PinnedChapterSnapshot>,

    /// Assignments retained in repository order without copying grouping keys.
    assignments: Vec<AssignmentInfo>,
}

impl ComicListData {
    /// Owns the models loaded for one aligned comic-list response.
    pub const fn new(
        comics: Vec<ComicInfo>,
        pinned_chapters: Option<PinnedChapterSnapshot>,
        assignments: Vec<AssignmentInfo>,
    ) -> Self {
        //
        Self {
            comics,
            pinned_chapters,
            assignments,
        }
    }
}

/// Resolves one comic model and every included object-backed model.
pub async fn comic_info_view<C, R, O>(
    repo: &R,
    obj_dept: &O,
    model: ComicInfo,
) -> BaseRest<ComicInfoView>
where
    C: Context,
    R: for<'a> Run<ListPinnedChapterInfos<'a>, Error = BaseError>
        + for<'a> Run<ListFirstPageInfos<'a>, Error = BaseError>
        + Sync,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<PageImage, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let mut ids = ObjViewIds::default();

    ids.collect_comics(std::slice::from_ref(&model));

    let mut snapshot =
        ObjViewSnapshot::load_with_comic_fallbacks(repo, obj_dept, ids, None)
            .await?;

    accept(snapshot.comic(model))
}

/// Resolves chapter models from one request-scoped object URL snapshot.
pub async fn chapter_info_views<C, R, O>(
    repo: &R,
    obj_dept: &O,
    models: Vec<ChapterInfo>,
) -> BaseRest<Vec<ChapterInfoView>>
where
    C: Context,
    R: for<'a> Run<ListPinnedChapterInfos<'a>, Error = BaseError>
        + for<'a> Run<ListFirstPageInfos<'a>, Error = BaseError>
        + Sync,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<PageImage, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let mut ids = ObjViewIds::default();

    ids.collect_chapters(&models);

    let mut snapshot =
        ObjViewSnapshot::load_with_comic_fallbacks(repo, obj_dept, ids, None)
            .await?;

    accept(
        models
            .into_iter()
            .map(|model| snapshot.chapter(model))
            .collect(),
    )
}

/// Resolves one assignment model and its included models.
///
/// Uses only object metadata; comic-cover page fallbacks are not loaded.
pub async fn assignment_info_view<C, O>(
    obj_dept: &O,
    model: AssignmentInfo,
) -> BaseRest<AssignmentInfoView>
where
    C: Context,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let mut ids = ObjViewIds::default();

    ids.collect_assignments(std::slice::from_ref(&model));

    let mut snapshot = ObjViewSnapshot::load(obj_dept, ids).await?;

    accept(snapshot.assignment(model))
}

/// Resolves assignment models from one request-scoped object URL snapshot.
pub async fn assignment_info_views<C, R, O>(
    repo: &R,
    obj_dept: &O,
    models: Vec<AssignmentInfo>,
) -> BaseRest<Vec<AssignmentInfoView>>
where
    C: Context,
    R: for<'a> Run<ListPinnedChapterInfos<'a>, Error = BaseError>
        + for<'a> Run<ListFirstPageInfos<'a>, Error = BaseError>
        + Sync,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<PageImage, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let mut ids = ObjViewIds::default();

    ids.collect_assignments(&models);

    let mut snapshot =
        ObjViewSnapshot::load_with_comic_fallbacks(repo, obj_dept, ids, None)
            .await?;

    accept(
        models
            .into_iter()
            .map(|model| snapshot.assignment(model))
            .collect(),
    )
}

/// Resolves all comic-list relations together and aligns them with comic order.
pub async fn comic_list_val<C, R, O>(
    repo: &R,
    obj_dept: &O,
    data: ComicListData,
) -> BaseRest<ListComicInfosVal>
where
    C: Context,
    R: for<'a> Run<ListPinnedChapterInfos<'a>, Error = BaseError>
        + for<'a> Run<ListFirstPageInfos<'a>, Error = BaseError>
        + Sync,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<PageImage, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let (mut snapshot, assignment_positions) = {
        //
        let pinned_infos = data
            .pinned_chapters
            .as_ref()
            .map(PinnedChapterSnapshot::infos)
            .unwrap_or_default();

        let positions_by_chapter_id = pinned_infos
            .iter()
            .enumerate()
            .map(|(position, chapter)| (chapter.id.as_str(), position))
            .collect::<HashMap<_, _>>();

        let assignment_positions = data
            .assignments
            .iter()
            .map(|assignment| {
                //
                positions_by_chapter_id
                    .get(assignment.chapter_id.as_str())
                    .copied()
            })
            .collect::<Vec<_>>();

        let mut ids = ObjViewIds::default();

        ids.collect_comics(&data.comics);

        ids.collect_chapters(pinned_infos);

        ids.collect_assignments(
            data.assignments
                .iter()
                .zip(&assignment_positions)
                .filter_map(|(assignment, position)| {
                    position.map(|_| assignment)
                }),
        );

        let pinned_lookup = data
            .pinned_chapters
            .as_ref()
            .map(|pinned| pinned.lookup(&data.comics));

        let snapshot = ObjViewSnapshot::load_with_comic_fallbacks(
            repo,
            obj_dept,
            ids,
            pinned_lookup.as_ref(),
        )
        .await?;

        (snapshot, assignment_positions)
    };

    let ComicListData {
        comics,
        pinned_chapters,
        assignments,
    } = data;

    let comic_count = comics.len();

    let (pinned_infos, positions) = pinned_chapters.map_or_else(
        || (Vec::new(), vec![None; comic_count]),
        PinnedChapterSnapshot::into_parts,
    );

    // Consume URL occurrences in the same order used during discovery.
    let comics = comics
        .into_iter()
        .map(|comic| snapshot.comic(comic))
        .collect();

    let mut pinned_views = pinned_infos
        .into_iter()
        .map(|chapter| Some(snapshot.chapter(chapter)))
        .collect::<Vec<_>>();

    let mut assignment_groups = (0..pinned_views.len())
        .map(|_| Vec::new())
        .collect::<Vec<_>>();

    for (assignment, position) in
        assignments.into_iter().zip(assignment_positions)
    {
        //
        let Some(group) =
            position.and_then(|position| assignment_groups.get_mut(position))
        else {
            continue;
        };

        group.push(snapshot.assignment(assignment));
    }

    let mut pinned_chapters = Vec::with_capacity(comic_count);

    let mut pinned_chapter_assignments = Vec::with_capacity(comic_count);

    for position in positions {
        //
        let chapter = position
            .and_then(|position| pinned_views.get_mut(position))
            .and_then(Option::take);

        let assignments = match chapter.as_ref() {
            //
            Some(_) => position
                .and_then(|position| assignment_groups.get_mut(position))
                .map(std::mem::take)
                .unwrap_or_default(),

            None => Vec::new(),
        };

        pinned_chapters.push(chapter);

        pinned_chapter_assignments.push(assignments);
    }

    accept(ListComicInfosVal {
        comics,
        pinned_chapters,
        pinned_chapter_assignments,
    })
}
