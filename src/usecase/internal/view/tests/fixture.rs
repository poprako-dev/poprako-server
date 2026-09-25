use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use poprako_obj_dept::key::ObjKey;
use poprako_obj_dept::model::meta::ObjMeta;
use poprako_obj_dept::model::url::ObjUrls;
use poprako_obj_dept::oper::{GenObjUrls, ListObjMetas};
use poprako_obj_dept::rest::ObjDeptError;
use poprako_orchestra::{Context, Level, Run, Step};
use time::OffsetDateTime;
use url::Url;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::UserInfo;
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::part::repo::oper::chapter::ListPinnedChapterInfos;
use crate::part::repo::oper::page::ListFirstPageInfos;
use crate::result::{BaseError, accept};
use crate::value::chapter::mask::StageMask;
use crate::value::role::RoleMask;

/// Isolation marker for recording object operations.
pub struct TestLevel;

impl Level for TestLevel {}

/// Context used only to satisfy existing object-view capabilities.
pub struct TestContext;

impl Context for TestContext {
    type Level = TestLevel;
}

/// Records object batches and simulates missing objects or failures.
#[derive(Default)]
pub struct TestObjDept {
    calls: Mutex<HashMap<&'static str, Vec<Vec<String>>>>,
    failures: Mutex<HashSet<&'static str>>,
    omissions: Mutex<HashSet<(&'static str, String)>>,
}

impl TestObjDept {
    /// Reports whether rendering performed any object operation.
    pub fn is_empty(&self) -> bool {
        self.calls.lock().unwrap().is_empty()
    }

    /// Returns every identifier batch requested for an object operation.
    pub fn calls(&self, operation: &'static str) -> Vec<Vec<String>> {
        self.calls
            .lock()
            .unwrap()
            .get(operation)
            .cloned()
            .unwrap_or_default()
    }

    /// Makes the selected object operation return an infrastructure error.
    pub fn fail(&self, operation: &'static str) {
        self.failures.lock().unwrap().insert(operation);
    }

    /// Removes one object from the metadata response.
    pub fn omit(&self, operation: &'static str, id: &str) {
        self.omissions
            .lock()
            .unwrap()
            .insert((operation, id.into()));
    }

    // Records one requested object batch.
    fn record(&self, operation: &'static str, ids: Vec<String>) {
        self.calls
            .lock()
            .unwrap()
            .entry(operation)
            .or_default()
            .push(ids);
    }

    // Checks whether this operation is configured to fail.
    fn fails(&self, operation: &'static str) -> bool {
        self.failures.lock().unwrap().contains(operation)
    }

    // Checks whether this object should be absent from metadata.
    fn is_omitted(&self, operation: &'static str, id: &str) -> bool {
        self.omissions
            .lock()
            .unwrap()
            .contains(&(operation, id.into()))
    }
}

// Implements the existing object-view capabilities for each object marker.
macro_rules! impl_obj_dept_view {
    ($marker:ty, $list_operation:literal, $url_operation:literal) => {
        impl<'a> Run<ListObjMetas<'a, $marker>> for TestObjDept {
            type Error = ObjDeptError;

            // Executes an object operation while recording its requested batch.
            async fn run(
                &self,
                oper: &ListObjMetas<'a, $marker>,
            ) -> Result<HashMap<String, ObjMeta>, Self::Error> {
                self.record(
                    $list_operation,
                    oper.ids.iter().map(|id| (*id).to_owned()).collect(),
                );

                if self.fails($list_operation) {
                    return Err(ObjDeptError::Unrecoverable {
                        message: $list_operation.into(),
                    });
                }

                Ok(oper
                    .ids
                    .iter()
                    .copied()
                    .filter(|id| !self.is_omitted($list_operation, id))
                    .map(|id| {
                        (
                            id.to_owned(),
                            ObjMeta {
                                key: ObjKey {
                                    id: id.to_owned(),
                                    ver: 1,
                                    image: format!("test/{}-1.png", id),
                                },
                                is_avail: true,
                                hash: vec![1; 32],
                                ext: "png".into(),
                            },
                        )
                    })
                    .collect())
            }
        }

        impl<'a> Run<GenObjUrls<'a, $marker>> for TestObjDept {
            type Error = ObjDeptError;

            // Executes an object operation while recording its requested batch.
            async fn run(
                &self,
                oper: &GenObjUrls<'a, $marker>,
            ) -> Result<HashMap<String, ObjUrls>, Self::Error> {
                let mut ids = oper.metas.keys().cloned().collect::<Vec<_>>();

                ids.sort_unstable();

                self.record($url_operation, ids);

                if self.fails($url_operation) {
                    return Err(ObjDeptError::Unrecoverable {
                        message: $url_operation.into(),
                    });
                }

                Ok(oper
                    .metas
                    .keys()
                    .map(|id| {
                        let origin_url =
                            Url::parse(&format!("https://obj.test/{id}"))
                                .unwrap();

                        let thumbnail_url = Url::parse(&format!(
                            "https://obj.test/thumbnail/{id}"
                        ))
                        .unwrap();

                        (
                            id.clone(),
                            ObjUrls {
                                origin_url: Some(origin_url),
                                optimized_url: None,
                                thumbnail_url: Some(thumbnail_url),
                            },
                        )
                    })
                    .collect())
            }
        }

        impl<'a> Step<ListObjMetas<'a, $marker>, TestContext> for TestObjDept {
            type Level = TestLevel;
            type Error = ObjDeptError;

            // Uses the same metadata behavior inside the test context.
            async fn step(
                &self,
                _context: &mut TestContext,
                oper: &ListObjMetas<'a, $marker>,
            ) -> Result<HashMap<String, ObjMeta>, Self::Error> {
                Run::run(self, oper).await
            }
        }
    };
}

impl_obj_dept_view!(ComicCover, "cover-list", "cover-urls");
impl_obj_dept_view!(PageImage, "page-list", "page-urls");
impl_obj_dept_view!(TeamAvatar, "team-list", "team-urls");
impl_obj_dept_view!(UserAvatar, "user-list", "user-urls");

/// Records cover fallback queries over explicitly supplied pinned chapters.
pub struct TestRepo {
    pinned_chapters: Vec<ChapterInfo>,
    pinned_chapter_calls: Mutex<Vec<Vec<String>>>,
    first_page_calls: Mutex<Vec<Vec<String>>>,
}

impl Default for TestRepo {
    // Provides one pinned chapter for cover fallback.
    fn default() -> Self {
        Self::with_chapters(vec![fallback_chapter_info()])
    }
}

impl TestRepo {
    /// Builds the fallback repository with known pinned chapters.
    pub fn with_chapters(pinned_chapters: Vec<ChapterInfo>) -> Self {
        Self {
            pinned_chapters,
            pinned_chapter_calls: Mutex::default(),
            first_page_calls: Mutex::default(),
        }
    }

    /// Returns the chapter batches queried for first pages.
    pub fn first_page_calls(&self) -> Vec<Vec<String>> {
        self.first_page_calls.lock().unwrap().clone()
    }

    /// Returns the comic batches queried for pinned chapters.
    pub fn pinned_chapter_calls(&self) -> Vec<Vec<String>> {
        self.pinned_chapter_calls.lock().unwrap().clone()
    }
}

impl<'a> Run<ListPinnedChapterInfos<'a>> for TestRepo {
    type Error = BaseError;

    // Returns the matching domain models and records the fallback query.
    async fn run(
        &self,
        oper: &ListPinnedChapterInfos<'a>,
    ) -> Result<Vec<ChapterInfo>, Self::Error> {
        self.pinned_chapter_calls
            .lock()
            .unwrap()
            .push(oper.comic_ids.iter().map(|id| (*id).to_owned()).collect());

        accept(
            self.pinned_chapters
                .iter()
                .filter(|chapter| {
                    oper.comic_ids.contains(&chapter.comic_id.as_str())
                })
                .cloned()
                .collect(),
        )
    }
}

impl<'a> Run<ListFirstPageInfos<'a>> for TestRepo {
    type Error = BaseError;

    // Returns the matching domain models and records the fallback query.
    async fn run(
        &self,
        oper: &ListFirstPageInfos<'a>,
    ) -> Result<Vec<PageInfo>, Self::Error> {
        self.first_page_calls
            .lock()
            .unwrap()
            .push(oper.chapter_ids.iter().map(|id| (*id).to_owned()).collect());

        let page_info = fallback_page_info();

        match oper.chapter_ids.contains(&page_info.chapter_id.as_str()) {
            true => accept(vec![page_info]),
            false => accept(Vec::new()),
        }
    }
}

/// Builds a fully included assignment with repeated user and comic relations.
pub fn assignment_info() -> AssignmentInfo {
    let created_at = OffsetDateTime::now_utc();

    let user_info = UserInfo {
        id: "user-1".into(),
        qid: "qid-1".into(),
        nickname: "User".into(),
        is_sadmin: false,
        last_active_at: created_at,
        created_at,
        updated_at: created_at,
    };

    let team_info = TeamInfo {
        id: "team-1".into(),
        name: "Team".into(),
        description: String::new(),
        created_at,
        updated_at: created_at,
    };

    let comic_info = ComicInfo {
        id: "comic-1".into(),
        workset_id: "workset-1".into(),
        index: 0,
        title: "Comic".into(),
        author: "Author".into(),
        description: None,
        chapter_count: 1,
        creator_id: user_info.id.clone(),
        workset: None,
        team: Some(team_info),
        creator: Some(user_info.clone()),
        last_active_at: created_at,
        archived_at: None,
        created_at,
        updated_at: created_at,
    };

    let chapter_info = ChapterInfo {
        id: "chapter-1".into(),
        comic_id: comic_info.id.clone(),
        comic: Some(comic_info),
        is_pinned: true,
        index: 0,
        subtitle: "Chapter".into(),
        page_count: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: StageMask::try_from(0).unwrap(),
        creator_id: user_info.id.clone(),
        creator: Some(user_info.clone()),
        created_at,
        updated_at: created_at,
    };

    AssignmentInfo {
        id: "assignment-1".into(),
        chapter_id: chapter_info.id.clone(),
        user_id: user_info.id.clone(),
        user: Some(user_info),
        chapter: Some(chapter_info),
        roles: RoleMask::try_from(1).unwrap(),
        created_at,
        updated_at: created_at,
    }
}

// Builds the pinned chapter returned for the default comic.
fn fallback_chapter_info() -> ChapterInfo {
    let mut assignment_info = assignment_info();

    let mut chapter_info = assignment_info.chapter.take().unwrap();

    chapter_info.comic = None;

    chapter_info.creator = None;

    chapter_info
}

// Builds the first page for the default pinned chapter.
fn fallback_page_info() -> PageInfo {
    let created_at = OffsetDateTime::now_utc();

    PageInfo {
        id: "page-1".into(),
        chapter_id: "chapter-1".into(),
        index: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at,
        updated_at: created_at,
    }
}
