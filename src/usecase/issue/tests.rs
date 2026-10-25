#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

//! Whole-chapter current review import and read permissions.

mod editor;

use super::*;

use time::OffsetDateTime;

use crate::data::instr::issue::{IssueInstr, IssueRectInstr, PageIssuesInstr};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::page_source::PageSourceInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::part::repo::oper::page_source::DeletePageSources;
use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::usecase::comic_archive as comic_archive_usecase;
use crate::value::chapter::mask::StageMask;
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::role::{RoleField, RoleMask};

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn comic(id: &str) -> ComicInfo {
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),

        workset_id: "workset--00000000001".into(),

        index: 0,
        title: "Pop Comic".into(),
        author: "author".into(),
        description: None,
        chapter_count: 1,

        creator_id: "user--00000000001".into(),

        workset: None,
        team: None,
        creator: None,

        last_active_at: time,
        archived_at: None,

        created_at: time,
        updated_at: time,
    }
}

fn workset(id: &str) -> WorksetInfo {
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),

        team_id: "team--00000000001".into(),

        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 1,

        created_at: time,
        updated_at: time,
    }
}

fn chapter(id: &str) -> ChapterInfo {
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        confirmed_artwork_ver: None,
        id: id.into(),

        comic_id: "comic--00000000001".into(),
        is_pinned: true,

        index: 3,
        subtitle: "Arrival".into(),

        page_source_count: 2,
        total_unit_count: 2,
        translated_unit_count: 2,
        proofread_unit_count: 1,

        stages: StageMask::try_from(0u32).unwrap(),

        creator_id: "user--00000000001".into(),

        comic: None,
        creator: None,

        created_at: time,
        updated_at: time,
    }
}

fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> AssignmentInfo {
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: ["assignment-", chapter_id, "-", user_id].concat(),

        chapter_id: chapter_id.into(),

        user_id: user_id.into(),

        user: None,
        chapter: None,

        roles: role_mask,

        created_at: time,
        updated_at: time,
    }
}

fn member(user_id: &str) -> MemberInfo {
    MemberInfo {
        id: format!("member-{user_id}"),

        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),

        team_id: "team--00000000001".into(),

        user: None,
        team: None,

        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn seed(role: RoleField) -> Mock {
    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001"));

    mock.seed_comic(comic("comic--00000000001"));

    mock.seed_chapter(chapter("chapter--00000000001"));

    match role {
        RoleField::ADMIN => {
            let mut member_info = member("user--00000000001");

            member_info.roles = RoleMask::from(RoleField::ADMIN);

            mock.seed_member(member_info);
        }

        _ => mock.seed_assignment(assignment(
            "chapter--00000000001",
            "user--00000000001",
            RoleMask::from(role),
        )),
    }

    mock
}

fn review_mock(role: RoleField) -> Mock {
    let mock = seed(role);

    for index in [1, 0] {
        let now = OffsetDateTime::now_utc();

        mock.state.lock().unwrap().page_artworks.push(
            crate::model::read::proj::page_artwork::PageArtworkInfo {
                id: format!("page_artwork--{index:011}"),
                chapter_id: "chapter--00000000001".into(),
                index: index + 3,
                raw_ident: None,
                created_at: now,
                updated_at: now,
            },
        );

        mock.seed_page(PageSourceInfo {
            id: format!("page_source--{index:011}"),
            chapter_id: "chapter--00000000001".into(),
            index,
            total_unit_count: 0,
            translated_unit_count: 0,
            proofread_unit_count: 0,
            created_at: now,
            updated_at: now,
        });
    }

    mock
}

fn input(note: &str) -> ImportChapterIssuesInstr {
    ImportChapterIssuesInstr {
        pages: vec![
            PageIssuesInstr {
                page_artwork_id: "page_artwork--00000000000".into(),
                issues: vec![IssueInstr {
                    variant: "\u{2003}custom issue\u{2003}".into(),
                    rect: Some(IssueRectInstr {
                        x_coord: 0.1,
                        y_coord: 0.2,
                        width: 0.3,
                        height: 0.1,
                    }),
                    note: note.into(),
                }],
            },
            PageIssuesInstr {
                page_artwork_id: "page_artwork--00000000001".into(),
                issues: vec![],
            },
        ],
    }
}

async fn submit(
    mock: &Mock,
    instr: ImportChapterIssuesInstr,
) -> BaseRest<ImportChapterIssuesVal> {
    import_issue(
        (mock, mock),
        token("user--00000000001"),
        "chapter--00000000001".into(),
        instr,
    )
    .await
}

// review_replace(import_issue)(positive): Explicit composite IDs map each input, replacing the single review and renewing identities.
#[tokio::test]
async fn import_replaces_whole_chapter_and_preserves_text() {
    let mock = review_mock(RoleField::REVIEWER);

    let result = submit(&mock, input("  line one\nline two  "))
        .await
        .unwrap();

    assert_eq!(
        (result.imported_page_count, result.imported_issue_count),
        (2, 1)
    );

    let first = mock.snapshot().issues.first().unwrap().clone();

    assert_eq!(first.page_artwork_id, "page_artwork--00000000000");

    assert_eq!(first.index, 0);

    assert_eq!(first.variant, "custom issue");

    assert_eq!(first.note, "  line one\nline two  ");

    let issues = list_infos::<MockContext, _>(
        (&mock,),
        token("user--00000000001"),
        "chapter--00000000001".into(),
    )
    .await
    .unwrap();

    assert_eq!(issues.len(), 1);

    submit(&mock, input("")).await.unwrap();

    let second = mock.snapshot().issues.first().unwrap().clone();

    assert_ne!(first.id, second.id);

    assert_eq!(second.note, "");

    submit(&mock, ImportChapterIssuesInstr { pages: vec![] })
        .await
        .unwrap();

    assert!(mock.snapshot().issues.is_empty());

    let chapter = mock.snapshot().chapters.first().unwrap().clone();

    assert_eq!(chapter.total_unit_count, 2);

    assert_eq!(chapter.stages.get_phase(Stage::Review), StagePhase::Pending);

    mock.state.lock().unwrap().pages.clear();

    let mut instr = input("independent review");

    instr.pages.pop();

    assert_eq!(submit(&mock, instr).await.unwrap().imported_page_count, 1);

    assert_eq!(
        mock.snapshot().issues.first().unwrap().page_artwork_id,
        "page_artwork--00000000000"
    );
}

// review_geometry(import_issue)(positive): normalized whole-page rectangles are optional.
#[tokio::test]
async fn import_accepts_all_optional_geometry_combinations() {
    let mock = review_mock(RoleField::REVIEWER);

    let mut instr = input("note");

    let page = instr.pages.first_mut().unwrap();

    page.issues.clear();

    for has_rect in [false, true] {
        page.issues.push(IssueInstr {
            variant: "open category".into(),
            rect: has_rect.then_some(IssueRectInstr {
                x_coord: 0.0,
                y_coord: 0.0,
                width: 1.0,
                height: 1.0,
            }),
            note: String::new(),
        });
    }

    submit(&mock, instr).await.unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(
        snapshot
            .issues
            .iter()
            .map(|issue| issue.index)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
}

// review_validation(import_issue)(negative): invalid geometry and blank fields preserve the old review.
#[tokio::test]
async fn rejected_input_preserves_current_review() {
    let mock = review_mock(RoleField::REVIEWER);

    submit(&mock, input("baseline")).await.unwrap();

    let baseline = mock.snapshot().issues;

    for (x_coord, y_coord, width, height) in [
        (-0.1, 0.0, 0.1, 0.1),
        (0.0, -0.1, 0.1, 0.1),
        (0.0, 0.0, 0.0, 0.1),
        (0.0, 0.0, 0.1, -0.1),
        (0.9, 0.0, 0.2, 0.1),
        (0.0, 0.9, 0.1, 0.2),
        (f64::NAN, 0.0, 0.1, 0.1),
        (0.0, 0.0, f64::INFINITY, 0.1),
    ] {
        let mut instr = input("invalid");

        instr
            .pages
            .first_mut()
            .unwrap()
            .issues
            .first_mut()
            .unwrap()
            .rect = Some(IssueRectInstr {
            x_coord,
            y_coord,
            width,
            height,
        });

        assert!(submit(&mock, instr).await.is_err());

        assert_eq!(mock.snapshot().issues, baseline);
    }

    let mut instr = input("invalid");

    instr
        .pages
        .first_mut()
        .unwrap()
        .issues
        .first_mut()
        .unwrap()
        .variant = "\u{2003}\n\t".into();

    assert!(submit(&mock, instr).await.is_err());

    assert_eq!(mock.snapshot().issues, baseline);
}

// review_permissions(import_issue, list_infos)(negative): only an assigned REVIEWER may write; team membership or chapter assignment may read.
#[tokio::test]
async fn review_permissions_and_published_freeze() {
    for role in [
        RoleField::ADMIN,
        RoleField::TRANSLATOR,
        RoleField::TYPESETTER,
        RoleField::PROOFREADER,
    ] {
        let mock = review_mock(role);

        assert!(matches!(
            submit(&mock, input("denied")).await,
            Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                ..
            })
        ));
    }

    let mock = review_mock(RoleField::REVIEWER);

    submit(&mock, input("current")).await.unwrap();

    mock.seed_member(member("member"));

    assert!(
        list_infos::<MockContext, _>(
            (&mock,),
            token("member"),
            "chapter--00000000001".into()
        )
        .await
        .is_ok()
    );

    assert!(
        list_infos::<MockContext, _>(
            (&mock,),
            token("outsider"),
            "chapter--00000000001".into()
        )
        .await
        .is_err()
    );

    assert!(
        list_infos::<MockContext, _>(
            (&mock,),
            token("user--00000000001"),
            "missing".into()
        )
        .await
        .is_err()
    );

    mock.state
        .lock()
        .unwrap()
        .chapters
        .first_mut()
        .unwrap()
        .stages = StageMask::try_from(0u32)
        .unwrap()
        .try_set_phase(Stage::Publish, StagePhase::Completed)
        .unwrap();

    assert!(submit(&mock, input("frozen")).await.is_err());

    assert_eq!(mock.snapshot().issues.first().unwrap().note, "current");
}

// review_atomicity(import_issue)(negative): a failure after replacement rolls back the entire current review.
#[tokio::test]
async fn replacement_rolls_back_on_comic_touch_failure() {
    let mock = review_mock(RoleField::REVIEWER);

    submit(&mock, input("baseline")).await.unwrap();

    let baseline = mock.snapshot().issues;

    mock.state.lock().unwrap().comics.clear();

    assert!(submit(&mock, input("replacement")).await.is_err());

    assert_eq!(mock.snapshot().issues, baseline);
}

// review_concurrency(import_issue)(positive): concurrent replacements retain one complete batch without mixed review data.
#[tokio::test]
async fn concurrent_imports_keep_one_complete_review() {
    let mock = review_mock(RoleField::REVIEWER);

    let (first, second) = tokio::join!(
        submit(&mock, input("first")),
        submit(&mock, input("second"))
    );

    first.unwrap();

    second.unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.issues.len(), 1);

    assert!(matches!(
        snapshot.issues.first().unwrap().note.as_str(),
        "first" | "second"
    ));
}

// review_lifecycle(DeletePageSources, archive)(positive): PageSource deletion preserves details; successful archive preserves the current review and removes active issues.
#[tokio::test]
async fn archive_snapshot_and_page_delete_include_review() {
    let mock = review_mock(RoleField::REVIEWER);

    submit(&mock, input("archived note")).await.unwrap();

    mock.coord(async |context| {
        DeletePageSources::Ids {
            ids: &["page_source--00000000000".into()],
        }
        .step_on(&mock, context)
        .await
    })
    .await
    .unwrap();

    assert_eq!(mock.snapshot().issues.len(), 1);

    let mut admin = member("user--00000000001");

    admin.roles = RoleMask::from(RoleField::ADMIN);

    mock.seed_member(admin);

    {
        let mut state = mock.state.lock().unwrap();

        state.assignments.clear();

        state.chapters.first_mut().unwrap().stages = StageMask::try_from(0u32)
            .unwrap()
            .try_set_phase(Stage::Publish, StagePhase::Completed)
            .unwrap();
    }

    comic_archive_usecase::archive(
        (&mock, &mock, &mock),
        token("user--00000000001"),
        "comic--00000000001".into(),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.issues.is_empty());

    let payload: serde_json::Value = serde_json::from_str(
        &snapshot.comic_archives.first().unwrap().archived_payload,
    )
    .unwrap();

    let notes = payload
        .get("chapters")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|chapter| {
            chapter.get("page_artworks").unwrap().as_array().unwrap()
        })
        .flat_map(|page| page.get("issues").unwrap().as_array().unwrap())
        .map(|issue| issue.get("note").unwrap().as_str().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(notes, vec!["archived note"]);
}
