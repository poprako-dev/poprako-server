//! Object identifier discovery, URL snapshots, and nested model rendering.

use std::collections::{HashMap, VecDeque};

use poprako_orchestra::{Context, Run};

use poprako_obj_dept::ObjDeptView;

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
use crate::usecase::internal::view::obj_urls::{
    ObjUrlBatch, ObjUrlViews, load_obj_urls,
};

/// Borrowed object occurrences discovered in rendering order.
#[derive(Default)]
pub struct ObjViewIds<'a> {
    /// Comic-cover identifiers by comic.
    comic_covers: Vec<&'a str>,

    /// Team-avatar identifiers by team.
    team_avatars: Vec<&'a str>,

    /// User-avatar identifiers by user.
    user_avatars: Vec<&'a str>,
}

impl<'a> ObjViewIds<'a> {
    /// Adds every object identifier reachable from assignment models.
    pub fn collect_assignments<I>(&mut self, assignment_infos: I)
    where
        I: IntoIterator<Item = &'a AssignmentInfo>,
    {
        //
        for assignment_info in assignment_infos {
            self.collect_assignment(assignment_info);
        }
    }

    /// Adds every object identifier reachable from chapter models.
    pub fn collect_chapters<I>(&mut self, chapter_infos: I)
    where
        I: IntoIterator<Item = &'a ChapterInfo>,
    {
        //
        for chapter_info in chapter_infos {
            self.collect_chapter(chapter_info);
        }
    }

    /// Adds every object identifier reachable from comic models.
    pub fn collect_comics<I>(&mut self, comic_infos: I)
    where
        I: IntoIterator<Item = &'a ComicInfo>,
    {
        //
        for comic_info in comic_infos {
            self.collect_comic(comic_info);
        }
    }

    // Collects object identifiers reachable from one assignment model.
    fn collect_assignment(&mut self, assignment_info: &'a AssignmentInfo) {
        //
        if let Some(user_info) = assignment_info.user.as_ref() {
            self.collect_user(user_info);
        }

        if let Some(chapter_info) = assignment_info.chapter.as_ref() {
            self.collect_chapter(chapter_info);
        }
    }

    // Collects object identifiers reachable from one chapter model.
    fn collect_chapter(&mut self, chapter_info: &'a ChapterInfo) {
        //
        if let Some(comic_info) = chapter_info.comic.as_ref() {
            self.collect_comic(comic_info);
        }

        if let Some(user_info) = chapter_info.creator.as_ref() {
            self.collect_user(user_info);
        }
    }

    // Collects object identifiers reachable from one comic model.
    fn collect_comic(&mut self, comic_info: &'a ComicInfo) {
        //
        self.comic_covers.push(&comic_info.id);

        if let Some(team_info) = comic_info.team.as_ref() {
            self.collect_team(team_info);
        }

        if let Some(user_info) = comic_info.creator.as_ref() {
            self.collect_user(user_info);
        }
    }

    // Collects one user avatar identifier.
    fn collect_user(&mut self, user_info: &'a UserInfo) {
        self.user_avatars.push(&user_info.id);
    }

    // Collects one team avatar identifier.
    fn collect_team(&mut self, team_info: &'a TeamInfo) {
        self.team_avatars.push(&team_info.id);
    }
}

/// Object URLs consumed once per response occurrence.
pub struct ObjViewSnapshot {
    /// Cover results in the same traversal order as the response models.
    comic_covers: VecDeque<Option<ObjUrlViews>>,

    /// Avatar batches retain only the handles still needed by the response.
    team_avatars: ObjUrlBatch,
    /// User-avatar handles awaiting their remaining consumers.
    user_avatars: ObjUrlBatch,
}

impl ObjViewSnapshot {
    /// Loads object occurrences without repository fallback reads.
    pub async fn load<C, O>(obj_dept: &O, ids: ObjViewIds<'_>) -> BaseRest<Self>
    where
        C: Context,
        O: ObjDeptView<ComicCover, C>
            + ObjDeptView<TeamAvatar, C>
            + ObjDeptView<UserAvatar, C>
            + Sync,
    {
        //
        Self::load_parts(
            obj_dept,
            &ids.comic_covers,
            ids.team_avatars,
            ids.user_avatars,
        )
        .await
    }

    /// Resolves fallback associations before releasing all borrowed model IDs.
    pub async fn load_with_comic_fallbacks<C, R, O>(
        repo: &R,
        obj_dept: &O,
        ids: ObjViewIds<'_>,
        pinned_chapters: Option<&HashMap<&str, Option<&ChapterInfo>>>,
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
        //
        let (mut snapshot, pages) = futures_util::try_join!(
            Self::load_parts(
                obj_dept,
                &ids.comic_covers,
                ids.team_avatars,
                ids.user_avatars
            ),
            PageLoader::load_ids_from_comics(
                repo,
                &ids.comic_covers,
                pinned_chapters
            ),
        )?;

        let (page_ids, positions) = pages.into_parts();

        let fallback_ids = snapshot
            .comic_covers
            .iter()
            .zip(&positions)
            .filter(|(cover, _)| cover.is_none())
            .filter_map(|(_, position)| {
                position.and_then(|position| page_ids.get(position))
            })
            .map(String::as_str)
            .collect();

        let mut page_urls =
            load_obj_urls::<_, _, PageImage>(obj_dept, fallback_ids).await?;

        for (cover, position) in snapshot.comic_covers.iter_mut().zip(positions)
        {
            //
            if cover.is_some() {
                continue;
            }

            if let Some(page_id) =
                position.and_then(|position| page_ids.get(position))
            {
                *cover = page_urls.take(page_id);
            }
        }

        accept(snapshot)
    }

    /// Renders an assignment and every included model without further I/O.
    pub fn assignment(
        &mut self,
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
    pub fn chapter(
        &mut self,
        mut chapter_info: ChapterInfo,
    ) -> ChapterInfoView {
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
    pub fn comic(&mut self, mut comic_info: ComicInfo) -> ComicInfoView {
        //
        let (cover_url, cover_thumbnail_url) =
            self.comic_covers.pop_front().flatten().unwrap_or_default();

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
    pub fn team(&mut self, team_info: TeamInfo) -> TeamInfoView {
        //
        let (avatar_url, avatar_thumbnail_url) =
            self.team_avatars.take(&team_info.id).unwrap_or_default();

        TeamInfoView::from_model(team_info, avatar_url, avatar_thumbnail_url)
    }

    /// Renders a user from the request snapshot without further I/O.
    pub fn user(&mut self, user_info: UserInfo) -> UserInfoView {
        //
        let (avatar_url, avatar_thumbnail_url) =
            self.user_avatars.take(&user_info.id).unwrap_or_default();

        UserInfoView::from_model(user_info, avatar_url, avatar_thumbnail_url)
    }

    // Keeps occurrence order separate from sorted metadata query identifiers.
    async fn load_parts<C, O>(
        obj_dept: &O,
        comic_ids: &[&str],
        team_ids: Vec<&str>,
        user_ids: Vec<&str>,
    ) -> BaseRest<Self>
    where
        C: Context,
        O: ObjDeptView<ComicCover, C>
            + ObjDeptView<TeamAvatar, C>
            + ObjDeptView<UserAvatar, C>
            + Sync,
    {
        //
        let (mut covers, team_avatars, user_avatars) = futures_util::try_join!(
            load_obj_urls::<_, _, ComicCover>(obj_dept, comic_ids.to_vec()),
            load_obj_urls::<_, _, TeamAvatar>(obj_dept, team_ids),
            load_obj_urls::<_, _, UserAvatar>(obj_dept, user_ids),
        )?;

        let comic_covers = comic_ids.iter().map(|id| covers.take(id)).collect();

        accept(Self {
            comic_covers,
            team_avatars,
            user_avatars,
        })
    }
}
