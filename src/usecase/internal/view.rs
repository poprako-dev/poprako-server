//! Request-scoped object URL hydration for nested response models.

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use poprako_orchestra::{Context, OperRun as _, Run};

use poprako_obj_dept::ObjDeptView;
use poprako_obj_dept::key::KeyMap;
use poprako_obj_dept::model::url::ObjUrls;
use poprako_obj_dept::oper::{GenObjUrls, ListObjMetas};

use crate::data::view::assignment::AssignmentInfoView;
use crate::data::view::chapter::ChapterInfoView;
use crate::data::view::comic::ComicInfoView;
use crate::data::view::team::TeamInfoView;
use crate::data::view::user::UserInfoView;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::UserInfo;
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::part::repo::oper::chapter::ListPinnedChapterInfos;
use crate::part::repo::oper::page::ListFirstPageInfos;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::internal::page::PageLoader;

/// Deduplicated object identifiers discovered in a complete include graph.
#[derive(Default)]
pub struct ObjViewIds {
    //
    /// Comic-cover identifiers by comic.
    comic_covers: HashSet<String>,

    /// Team-avatar identifiers by team.
    team_avatars: HashSet<String>,

    /// User-avatar identifiers by user.
    user_avatars: HashSet<String>,
}

impl ObjViewIds {
    /// Adds every object identifier reachable from assignment models.
    pub fn collect_assignments<'a, I>(&mut self, assignment_infos: I)
    where
        I: IntoIterator<Item = &'a AssignmentInfo>,
    {
        //
        for assignment_info in assignment_infos {
            self.collect_assignment(assignment_info);
        }
    }

    /// Adds every object identifier reachable from chapter models.
    pub fn collect_chapters<'a, I>(&mut self, chapter_infos: I)
    where
        I: IntoIterator<Item = &'a ChapterInfo>,
    {
        //
        for chapter_info in chapter_infos {
            self.collect_chapter(chapter_info);
        }
    }

    /// Adds every object identifier reachable from comic models.
    pub fn collect_comics<'a, I>(&mut self, comic_infos: I)
    where
        I: IntoIterator<Item = &'a ComicInfo>,
    {
        //
        for comic_info in comic_infos {
            self.collect_comic(comic_info);
        }
    }

    // Collects object identifiers reachable from one assignment model.
    fn collect_assignment(&mut self, assignment_info: &AssignmentInfo) {
        //
        if let Some(user_info) = assignment_info.user.as_ref() {
            self.collect_user(user_info);
        }

        if let Some(chapter_info) = assignment_info.chapter.as_ref() {
            self.collect_chapter(chapter_info);
        }
    }

    // Collects object identifiers reachable from one chapter model.
    fn collect_chapter(&mut self, chapter_info: &ChapterInfo) {
        //
        if let Some(comic_info) = chapter_info.comic.as_ref() {
            self.collect_comic(comic_info);
        }

        if let Some(user_info) = chapter_info.creator.as_ref() {
            self.collect_user(user_info);
        }
    }

    // Collects object identifiers reachable from one comic model.
    fn collect_comic(&mut self, comic_info: &ComicInfo) {
        //
        self.comic_covers.insert(comic_info.id.clone());

        if let Some(team_info) = comic_info.team.as_ref() {
            self.collect_team(team_info);
        }

        if let Some(user_info) = comic_info.creator.as_ref() {
            self.collect_user(user_info);
        }
    }

    // Collects one user avatar identifier.
    fn collect_user(&mut self, user_info: &UserInfo) {
        self.user_avatars.insert(user_info.id.clone());
    }

    // Collects one team avatar identifier.
    fn collect_team(&mut self, team_info: &TeamInfo) {
        self.team_avatars.insert(team_info.id.clone());
    }

    // Returns the sorted comic identifiers used for cover fallback lookup.
    fn comic_ids(&self) -> Vec<String> {
        //
        let mut comic_ids =
            self.comic_covers.iter().cloned().collect::<Vec<_>>();

        comic_ids.sort_unstable();

        comic_ids
    }
}

/// Object URLs loaded once for every marker present in a request include graph.
pub struct ObjViewSnapshot {
    //
    /// Comic-cover URLs by comic identifier.
    comic_covers: HashMap<String, ObjUrls>,

    /// First-page identifiers by comic for cover fallback.
    comic_fallback_pages: HashMap<String, String>,

    /// First-page image URLs by page identifier.
    page_images: HashMap<String, ObjUrls>,

    /// Team-avatar URLs by team identifier.
    team_avatars: HashMap<String, ObjUrls>,

    /// User-avatar URLs by user identifier.
    user_avatars: HashMap<String, ObjUrls>,
}

impl ObjViewSnapshot {
    /// Loads one metadata batch and one URL batch for each non-empty marker.
    pub async fn load<C, O>(obj_dept: &O, ids: ObjViewIds) -> BaseRest<Self>
    where
        C: Context,
        O: ObjDeptView<ComicCover, C>
            + ObjDeptView<TeamAvatar, C>
            + ObjDeptView<UserAvatar, C>
            + Sync,
    {
        let mut comic_cover_ids =
            ids.comic_covers.into_iter().collect::<Vec<_>>();

        let mut team_avatar_ids =
            ids.team_avatars.into_iter().collect::<Vec<_>>();

        let mut user_avatar_ids =
            ids.user_avatars.into_iter().collect::<Vec<_>>();

        comic_cover_ids.sort_unstable();

        team_avatar_ids.sort_unstable();

        user_avatar_ids.sort_unstable();

        let (comic_covers, team_avatars, user_avatars) = futures_util::try_join!(
            load_obj_urls::<C, O, ComicCover>(obj_dept, &comic_cover_ids),
            load_obj_urls::<C, O, TeamAvatar>(obj_dept, &team_avatar_ids),
            load_obj_urls::<C, O, UserAvatar>(obj_dept, &user_avatar_ids),
        )?;

        accept(Self {
            comic_covers,
            comic_fallback_pages: HashMap::new(),
            page_images: HashMap::new(),
            team_avatars,
            user_avatars,
        })
    }

    /// Loads nested object URLs and the comic-cover fallback relationship.
    pub async fn load_with_comic_fallbacks<C, R, O>(
        repo: &R,
        obj_dept: &O,
        ids: ObjViewIds,
    ) -> BaseRest<Self>
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
        let comic_ids = ids.comic_ids();

        let (mut snapshot, comic_fallback_pages) = futures_util::try_join!(
            Self::load::<C, O>(obj_dept, ids),
            PageLoader::load_ids_from_comics(repo, &comic_ids),
        )?;

        let mut page_ids =
            comic_fallback_pages.values().cloned().collect::<Vec<_>>();

        page_ids.sort_unstable();

        page_ids.dedup();

        snapshot.page_images =
            load_obj_urls::<C, O, PageImage>(obj_dept, &page_ids).await?;

        snapshot.comic_fallback_pages = comic_fallback_pages;

        accept(snapshot)
    }

    /// Renders an assignment and every included model without further I/O.
    pub fn assignment(
        &self,
        mut assignment_info: AssignmentInfo,
    ) -> AssignmentInfoView {
        //
        let user = assignment_info
            .user
            .take()
            .map(|user_info| self.user(user_info));

        let chapter = assignment_info
            .chapter
            .take()
            .map(|chapter_info| self.chapter(chapter_info));

        AssignmentInfoView::from_model(assignment_info, user, chapter)
    }

    /// Renders a chapter and every included model without further I/O.
    pub fn chapter(&self, mut chapter_info: ChapterInfo) -> ChapterInfoView {
        //
        let comic = chapter_info
            .comic
            .take()
            .map(|comic_info| self.comic(comic_info));

        let creator = chapter_info
            .creator
            .take()
            .map(|user_info| self.user(user_info));

        ChapterInfoView::from_model(chapter_info, comic, creator)
    }

    /// Renders a comic and every included model without further I/O.
    pub fn comic(&self, mut comic_info: ComicInfo) -> ComicInfoView {
        //
        let dedicated_cover_urls = self.comic_covers.get(&comic_info.id);

        let fallback_cover_urls = self
            .comic_fallback_pages
            .get(&comic_info.id)
            .and_then(|page_id| self.page_images.get(page_id));

        let (cover_url, cover_thumbnail_url) =
            resolved_obj_urls(dedicated_cover_urls.or(fallback_cover_urls));

        let team = comic_info.team.take().map(|team_info| self.team(team_info));

        let creator = comic_info
            .creator
            .take()
            .map(|user_info| self.user(user_info));

        ComicInfoView::from_model(
            comic_info,
            cover_url,
            cover_thumbnail_url,
            team,
            creator,
        )
    }

    /// Renders a team from the request snapshot without further I/O.
    pub fn team(&self, team_info: TeamInfo) -> TeamInfoView {
        //
        let (avatar_url, avatar_thumbnail_url) =
            resolved_urls(&self.team_avatars, &team_info.id);

        TeamInfoView::from_model(team_info, avatar_url, avatar_thumbnail_url)
    }

    /// Renders a user from the request snapshot without further I/O.
    pub fn user(&self, user_info: UserInfo) -> UserInfoView {
        //
        let (avatar_url, avatar_thumbnail_url) =
            resolved_urls(&self.user_avatars, &user_info.id);

        UserInfoView::from_model(user_info, avatar_url, avatar_thumbnail_url)
    }
}

// Resolves origin and thumbnail strings from one object URL value.
fn resolved_obj_urls(
    urls: Option<&ObjUrls>,
) -> (Option<String>, Option<String>) {
    //
    let Some(urls) = urls else {
        return (None, None);
    };

    (
        Some(urls.origin_url.to_string()),
        urls.thumbnail_url.as_ref().map(ToString::to_string),
    )
}

// Loads URLs for the supplied object marker identifiers.
async fn load_obj_urls<C, O, K>(
    obj_dept: &O,
    ids: &[String],
) -> BaseRest<HashMap<String, ObjUrls>>
where
    C: Context,
    K: KeyMap,
    O: ObjDeptView<K, C> + Sync,
{
    if ids.is_empty() {
        return accept(HashMap::new());
    }

    let obj_metas = ListObjMetas::<K>::new(ids)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    let obj_urls = GenObjUrls::<K>::new(&obj_metas)
        .run_on(obj_dept)
        .await
        .map_err(BaseError::from)?;

    accept(obj_urls)
}

// Resolves origin and thumbnail URLs for one identifier.
fn resolved_urls(
    urls_by_id: &HashMap<String, ObjUrls>,
    id: &str,
) -> (Option<String>, Option<String>) {
    resolved_obj_urls(urls_by_id.get(id))
}
