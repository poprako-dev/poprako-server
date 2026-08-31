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

use super::{ObjViewIds, ObjViewSnapshot};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::UserInfo;
use crate::part::obj_dept::{ComicCover, TeamAvatar, UserAvatar};
use crate::value::chapter::mask::StageMask;
use crate::value::role::RoleMask;

struct TestLevel;

impl Level for TestLevel {}

struct TestContext;

impl Context for TestContext {
    type Level = TestLevel;
}

#[derive(Default)]
struct TestObjDept {
    calls: Mutex<HashMap<&'static str, Vec<Vec<String>>>>,
    failures: Mutex<HashSet<&'static str>>,
}

impl TestObjDept {
    fn record(&self, operation: &'static str, ids: Vec<String>) {
        self.calls
            .lock()
            .unwrap()
            .entry(operation)
            .or_default()
            .push(ids);
    }

    fn calls(&self, operation: &'static str) -> Vec<Vec<String>> {
        self.calls
            .lock()
            .unwrap()
            .get(operation)
            .cloned()
            .unwrap_or_default()
    }

    fn fail(&self, operation: &'static str) {
        self.failures.lock().unwrap().insert(operation);
    }

    fn fails(&self, operation: &'static str) -> bool {
        self.failures.lock().unwrap().contains(operation)
    }
}

macro_rules! impl_obj_view {
    ($marker:ty, $list_operation:literal, $url_operation:literal) => {
        impl<'a> Run<ListObjMetas<'a, $marker>> for TestObjDept {
            type Error = ObjDeptError;

            async fn run(
                &self,
                oper: &ListObjMetas<'a, $marker>,
            ) -> Result<HashMap<String, ObjMeta>, Self::Error> {
                self.record($list_operation, oper.ids.to_vec());

                if self.fails($list_operation) {
                    return Err(ObjDeptError::Unrecoverable {
                        message: $list_operation.into(),
                    });
                }

                Ok(oper
                    .ids
                    .iter()
                    .map(|id| {
                        (
                            id.clone(),
                            ObjMeta {
                                key: ObjKey {
                                    id: id.clone(),
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
                                origin_url,
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

impl_obj_view!(ComicCover, "cover-list", "cover-urls");
impl_obj_view!(TeamAvatar, "team-list", "team-urls");
impl_obj_view!(UserAvatar, "user-list", "user-urls");

#[tokio::test]
async fn nested_repeated_models_load_once_per_object_marker() {
    let obj_dept = TestObjDept::default();

    let assignment_info = assignment_info();

    let mut ids = ObjViewIds::default();

    ids.collect_assignments([&assignment_info, &assignment_info]);

    let snapshot = ObjViewSnapshot::load::<TestContext, _>(&obj_dept, ids)
        .await
        .unwrap();

    let assignment_view = snapshot.assignment(assignment_info);

    let user_view = assignment_view.user.as_ref().unwrap();

    assert_eq!(
        user_view.avatar_thumbnail_url.as_deref(),
        Some("https://obj.test/thumbnail/user-1"),
    );

    let comic_view = assignment_view
        .chapter
        .as_ref()
        .and_then(|chapter_view| chapter_view.comic.as_ref())
        .unwrap();

    assert_eq!(
        comic_view.cover_thumbnail_url.as_deref(),
        Some("https://obj.test/thumbnail/comic-1"),
    );

    assert_eq!(
        comic_view
            .team
            .as_ref()
            .and_then(|team_view| team_view.avatar_thumbnail_url.as_deref()),
        Some("https://obj.test/thumbnail/team-1"),
    );

    assert_eq!(
        obj_dept.calls("cover-list"),
        vec![vec![String::from("comic-1")]]
    );
    assert_eq!(
        obj_dept.calls("cover-urls"),
        vec![vec![String::from("comic-1")]]
    );
    assert_eq!(
        obj_dept.calls("team-list"),
        vec![vec![String::from("team-1")]]
    );
    assert_eq!(
        obj_dept.calls("team-urls"),
        vec![vec![String::from("team-1")]]
    );
    assert_eq!(
        obj_dept.calls("user-list"),
        vec![vec![String::from("user-1")]]
    );
    assert_eq!(
        obj_dept.calls("user-urls"),
        vec![vec![String::from("user-1")]]
    );
}

#[tokio::test]
async fn empty_snapshot_performs_no_object_operations() {
    let obj_dept = TestObjDept::default();

    ObjViewSnapshot::load::<TestContext, _>(&obj_dept, ObjViewIds::default())
        .await
        .unwrap();

    assert!(obj_dept.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn partial_snapshot_skips_absent_markers() {
    let obj_dept = TestObjDept::default();

    let mut ids = ObjViewIds::default();

    ids.user_avatars.insert("user-1".into());

    ObjViewSnapshot::load::<TestContext, _>(&obj_dept, ids)
        .await
        .unwrap();

    assert_eq!(obj_dept.calls("user-list").len(), 1);
    assert_eq!(obj_dept.calls("user-urls").len(), 1);
    assert!(obj_dept.calls("cover-list").is_empty());
    assert!(obj_dept.calls("cover-urls").is_empty());
    assert!(obj_dept.calls("team-list").is_empty());
    assert!(obj_dept.calls("team-urls").is_empty());
}

#[tokio::test]
async fn multi_id_snapshot_deduplicates_and_sorts_each_batch() {
    let obj_dept = TestObjDept::default();

    let mut ids = ObjViewIds::default();

    ids.user_avatars.extend([
        "user-z".into(),
        "user-a".into(),
        "user-z".into(),
    ]);

    ObjViewSnapshot::load::<TestContext, _>(&obj_dept, ids)
        .await
        .unwrap();

    let expected_ids = vec![String::from("user-a"), String::from("user-z")];

    assert_eq!(obj_dept.calls("user-list"), vec![expected_ids.clone()]);
    assert_eq!(obj_dept.calls("user-urls"), vec![expected_ids]);
}

#[tokio::test]
async fn metadata_error_is_propagated_without_url_generation() {
    let obj_dept = TestObjDept::default();

    obj_dept.fail("user-list");

    let mut ids = ObjViewIds::default();

    ids.user_avatars.insert("user-1".into());

    let result = ObjViewSnapshot::load::<TestContext, _>(&obj_dept, ids).await;

    assert!(result.is_err());
    assert!(obj_dept.calls("user-urls").is_empty());
}

#[tokio::test]
async fn url_error_is_propagated_after_metadata_load() {
    let obj_dept = TestObjDept::default();

    obj_dept.fail("user-urls");

    let mut ids = ObjViewIds::default();

    ids.user_avatars.insert("user-1".into());

    let result = ObjViewSnapshot::load::<TestContext, _>(&obj_dept, ids).await;

    assert!(result.is_err());
    assert_eq!(obj_dept.calls("user-list").len(), 1);
    assert_eq!(obj_dept.calls("user-urls").len(), 1);
}

fn assignment_info() -> AssignmentInfo {
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
