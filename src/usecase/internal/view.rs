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

    let snapshot =
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

    let snapshot =
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

    let snapshot = ObjViewSnapshot::load(obj_dept, &ids).await?;

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

    let snapshot =
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
    comic_infos: Vec<ComicInfo>,
    pinned_chapter_snapshot: Option<PinnedChapterSnapshot>,
    pinned_chapter_assignment_infos: HashMap<String, Vec<AssignmentInfo>>,
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
    let pinned_chapter_infos = pinned_chapter_snapshot
        .as_ref()
        .map(PinnedChapterSnapshot::infos_by_comic_id);

    let mut obj_view_ids = ObjViewIds::default();

    obj_view_ids.collect_comics(&comic_infos);

    obj_view_ids.collect_chapters(
        pinned_chapter_infos
            .into_iter()
            .flat_map(|infos| infos.values()),
    );

    obj_view_ids.collect_assignments(
        pinned_chapter_assignment_infos.values().flatten(),
    );

    let obj_view_snapshot = ObjViewSnapshot::load_with_comic_fallbacks(
        repo,
        obj_dept,
        obj_view_ids,
        pinned_chapter_snapshot.as_ref(),
    )
    .await?;

    accept(build_list_val(
        &obj_view_snapshot,
        comic_infos,
        pinned_chapter_snapshot
            .map(PinnedChapterSnapshot::into_infos_by_comic_id)
            .unwrap_or_default(),
        pinned_chapter_assignment_infos,
    ))
}

// Build aligned comic, pinned-chapter, and assignment response vectors.
fn build_list_val(
    obj_view_snapshot: &ObjViewSnapshot,
    comic_infos: Vec<ComicInfo>,
    mut pinned_chapter_infos: HashMap<String, ChapterInfo>,
    mut assignment_infos_by_chapter: HashMap<String, Vec<AssignmentInfo>>,
) -> ListComicInfosVal {
    //
    let mut comic_info_views = Vec::with_capacity(comic_infos.len());

    let mut pinned_chapter_views = Vec::with_capacity(comic_infos.len());

    let mut pinned_chapter_assignment_views =
        Vec::with_capacity(comic_infos.len());

    for comic_info in comic_infos {
        //
        let chapter_info = pinned_chapter_infos.remove(&comic_info.id);

        let assignment_infos = chapter_info
            .as_ref()
            .and_then(|chapter_info| {
                assignment_infos_by_chapter.remove(&chapter_info.id)
            })
            .unwrap_or_default();

        let assignment_views = assignment_infos
            .into_iter()
            .map(|assignment_info| {
                obj_view_snapshot.assignment(assignment_info)
            })
            .collect();

        let pinned_chapter_view = chapter_info
            .map(|chapter_info| obj_view_snapshot.chapter(chapter_info));

        comic_info_views.push(obj_view_snapshot.comic(comic_info));

        pinned_chapter_views.push(pinned_chapter_view);

        pinned_chapter_assignment_views.push(assignment_views);
    }

    ListComicInfosVal {
        comics: comic_info_views,
        pinned_chapters: pinned_chapter_views,
        pinned_chapter_assignments: pinned_chapter_assignment_views,
    }
}
