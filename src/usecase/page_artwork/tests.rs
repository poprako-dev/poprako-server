#![allow(
    clippy::unwrap_used,
    reason = "Lifecycle fixtures and assertions fail immediately on violated invariants"
)]

//! Composite page identity, upload and review lifecycle tests.

// Targeted image allocation must preserve every sibling's metadata.
mod image;

use time::OffsetDateTime;

use crate::config::image::ImageConfig;
use crate::data::instr::issue::{
    ImportChapterIssuesInstr, IssueInstr, PageIssuesInstr,
};
use crate::data::instr::page_artwork::{
    AllocChapterPageArtworksInstr, AllocPageArtworkImageInstr,
    MarkPageArtworkImageUploadedInstr, PageArtworkImageInstr,
};
use crate::data::val::page_artwork::AllocChapterPageArtworksVal;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::{Mock, MockContext};
use crate::result::BaseRest;
use crate::usecase::comic_archive as comic_archive_usecase;
use crate::usecase::issue as issue_usecase;
use crate::usecase::page_artwork::{
    image as page_artwork_image_usecase,
    manifest as page_artwork_manifest_usecase,
};
use crate::value::chapter::mask::StageMask;
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::image::{ImageExt, ImageHash};
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
        workset_id: "workset-1".into(),
        index: 0,
        title: "Pop Comic".into(),
        author: "author".into(),
        description: None,
        chapter_count: 1,
        creator_id: "user-1".into(),
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
        team_id: "team-1".into(),
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
        comic_id: "comic-1".into(),
        is_pinned: true,
        index: 3,
        subtitle: "Arrival".into(),
        page_count: 2,
        total_unit_count: 2,
        translated_unit_count: 2,
        proofread_unit_count: 1,
        stages: StageMask::try_from(0u32).unwrap(),
        creator_id: "user-1".into(),
        comic: None,
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> AssignmentInfo {
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: format!("assignment-{}-{}", chapter_id, user_id),
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

        team_id: "team-1".into(),

        user: None,
        team: None,

        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn seed(role: RoleField) -> Mock {
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1"));

    mock.seed_comic(comic("comic-1"));

    mock.seed_chapter(chapter("chapter-1"));

    match role {
        RoleField::ADMIN => {
            let mut member_info = member("user-1");

            member_info.roles = RoleMask::from(RoleField::ADMIN);

            mock.seed_member(member_info);
        }

        _ => mock.seed_assignment(assignment(
            "chapter-1",
            "user-1",
            RoleMask::from(role),
        )),
    }

    mock
}

fn config() -> ImageConfig {
    ImageConfig {
        user_avatar_limit: 1,
        team_avatar_limit: 1,
        comic_cover_limit: 1,
        page_image_limit: 1,
    }
}

fn page(id: Option<&str>, byte: u8) -> PageArtworkImageInstr {
    PageArtworkImageInstr {
        page_artwork_id: id.map(str::to_owned),
        raw_ident: Some("same/file.psd".into()),
        image_hash: ImageHash::new([byte; 32]),
        new_byte_len: Some(100),
        ext: ImageExt::Png,
    }
}

async fn alloc(
    mock: &Mock,
    pages: Vec<PageArtworkImageInstr>,
) -> BaseRest<AllocChapterPageArtworksVal> {
    let instr = AllocChapterPageArtworksInstr { pages };

    page_artwork_manifest_usecase::alloc_chapter_page_artworks(
        (mock, mock, mock, &config()),
        token("user-1"),
        "chapter-1".into(),
        instr,
    )
    .await
}

async fn mark(mock: &Mock, id: &str, version: u32) -> BaseRest<()> {
    let instr = MarkPageArtworkImageUploadedInstr { image_ver: version };

    page_artwork_image_usecase::mark_image_uploaded(
        (mock, mock, mock),
        token("user-1"),
        id.into(),
        instr,
    )
    .await
}

async fn review(mock: &Mock, id: &str) {
    let instr = ImportChapterIssuesInstr {
        pages: vec![PageIssuesInstr {
            page_artwork_id: id.into(),
            issues: vec![IssueInstr {
                variant: "custom".into(),
                layer_name: Some("对白".into()),
                rect: None,
                note: "fix this text".into(),
            }],
        }],
    };

    issue_usecase::import_issue(
        (mock, mock),
        token("user-1"),
        "chapter-1".into(),
        instr,
    )
    .await
    .unwrap();
}

// composite_identity(alloc_chapter_page_artworks, mark_image_uploaded)(positive): same names and hashes create distinct pages without source images; reordering retains review targets.
#[tokio::test]
async fn composite_identity_is_independent_and_reorder_preserves_issues() {
    let mock = seed(RoleField::REVIEWER);

    let allocation =
        alloc(&mock, vec![page(None, 1), page(None, 1), page(None, 2)])
            .await
            .unwrap();

    let first = &allocation.pages.first().unwrap().page_artwork_id;

    let second = &allocation.pages.get(1).unwrap().page_artwork_id;

    assert_ne!(first, second);

    assert!(mock.snapshot().pages.is_empty());

    let views = super::list_infos::<MockContext, _, _>(
        (&mock, &mock),
        token("user-1"),
        "chapter-1".into(),
    )
    .await
    .unwrap();

    assert!(
        views
            .iter()
            .all(|view| !view.image_uploaded && view.image_url.is_none())
    );

    for allocated in &allocation.pages {
        mark(&mock, &allocated.page_artwork_id, allocated.image_ver)
            .await
            .unwrap();
    }

    review(&mock, first).await;

    let baseline = mock.snapshot().issues;

    let mut retained = page(Some(first), 1);

    retained.new_byte_len = None;

    let allocation = alloc(&mock, vec![page(Some(second), 1), retained])
        .await
        .unwrap();

    assert!(allocation.pages.iter().all(|page| page.slot.is_none()));

    assert_eq!(mock.snapshot().page_artworks.len(), 2);

    assert_eq!(mock.snapshot().issues, baseline);

    assert_eq!(
        mock.snapshot()
            .page_artworks
            .iter()
            .find(|info| info.id == *first)
            .unwrap()
            .index,
        1
    );

    assert_eq!(
        mock.snapshot()
            .chapters
            .first()
            .unwrap()
            .stages
            .get_phase(Stage::TypesetRedraw),
        StagePhase::Pending
    );
}

// composite_replacement(alloc_chapter_page_artworks, alloc_image, mark_image_uploaded)(positive): newer images and stale confirmations retain review issues; deletion removes them and queues object cleanup.
#[tokio::test]
async fn composite_replacement_preserves_review_until_explicit_deletion() {
    let mock = seed(RoleField::REVIEWER);

    let initial = alloc(&mock, vec![page(None, 1)]).await.unwrap();

    let first = initial.pages.first().unwrap();

    mark(&mock, &first.page_artwork_id, first.image_ver)
        .await
        .unwrap();

    review(&mock, &first.page_artwork_id).await;

    let baseline = mock.snapshot().issues;

    let instr = AllocPageArtworkImageInstr {
        raw_ident: Some("renamed.psd".into()),
        image_hash: ImageHash::new([2; 32]),
        new_byte_len: 100,
        ext: ImageExt::Png,
    };

    let replacement = page_artwork_image_usecase::alloc_image(
        (&mock, &mock, &mock, &config()),
        token("user-1"),
        first.page_artwork_id.clone(),
        instr,
    )
    .await
    .unwrap();

    assert!(replacement.image_ver > first.image_ver);

    assert!(
        mark(&mock, &first.page_artwork_id, first.image_ver)
            .await
            .is_err()
    );

    mark(&mock, &first.page_artwork_id, replacement.image_ver)
        .await
        .unwrap();

    mark(&mock, &first.page_artwork_id, replacement.image_ver)
        .await
        .unwrap();

    assert_eq!(mock.snapshot().issues, baseline);

    let page_artwork_infos = mock.snapshot().page_artworks;

    assert_eq!(
        page_artwork_infos.first().unwrap().raw_ident.as_deref(),
        Some("renamed.psd")
    );

    alloc(&mock, vec![]).await.unwrap();

    assert!(mock.snapshot().page_artworks.is_empty());

    assert!(mock.snapshot().issues.is_empty());

    assert!(
        !mock
            .snapshot()
            .objs
            .get("page_artwork_image")
            .unwrap()
            .contains_key(&first.page_artwork_id)
    );

    assert!(
        mock.snapshot()
            .obj_tasks
            .iter()
            .any(|(topic, _)| *topic == "page_artwork_image")
    );
}

// composite_permissions(alloc_chapter_page_artworks)(negative): ordinary assignees cannot mutate; designated workers and administrators can maintain composites.
#[tokio::test]
async fn composite_permissions_and_published_freeze() {
    for role in [
        RoleField::TYPESETTER,
        RoleField::REDRAWER,
        RoleField::REVIEWER,
        RoleField::ADMIN,
    ] {
        let mock = seed(role);

        assert!(alloc(&mock, vec![page(None, 1)]).await.is_ok());
    }

    let mock = seed(RoleField::TRANSLATOR);

    assert!(alloc(&mock, vec![page(None, 1)]).await.is_err());

    assert!(mock.snapshot().page_artworks.is_empty());

    let mock = seed(RoleField::REVIEWER);

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

    assert!(alloc(&mock, vec![page(None, 1)]).await.is_err());
}

// composite_atomicity(alloc_chapter_page_artworks)(negative): duplicate or foreign targets, missing lengths and later failures preserve manifest, objects and review together.
#[tokio::test]
async fn composite_manifest_failures_roll_back_all_state() {
    let mock = seed(RoleField::REVIEWER);

    let initial = alloc(&mock, vec![page(None, 1)]).await.unwrap();

    let first = initial.pages.first().unwrap();

    review(&mock, &first.page_artwork_id).await;

    let baseline = mock.snapshot();

    for pages in [
        vec![
            page(Some(&first.page_artwork_id), 1),
            page(Some(&first.page_artwork_id), 2),
        ],
        vec![page(Some("foreign-page"), 1)],
    ] {
        assert!(alloc(&mock, pages).await.is_err());
    }

    let mut invalid = page(None, 3);

    invalid.new_byte_len = None;

    assert!(alloc(&mock, vec![invalid]).await.is_err());

    assert_eq!(mock.snapshot().page_artworks, baseline.page_artworks);

    assert_eq!(mock.snapshot().issues, baseline.issues);

    assert_eq!(mock.snapshot().obj_tasks.len(), baseline.obj_tasks.len());

    mock.state.lock().unwrap().comics.clear();

    assert!(
        alloc(&mock, vec![page(Some(&first.page_artwork_id), 4)])
            .await
            .is_err()
    );

    assert_eq!(mock.snapshot().page_artworks, baseline.page_artworks);

    assert_eq!(mock.snapshot().issues, baseline.issues);

    let instr = ImportChapterIssuesInstr {
        pages: vec![PageIssuesInstr {
            page_artwork_id: "unknown".into(),
            issues: vec![],
        }],
    };

    assert!(
        issue_usecase::import_issue(
            (&mock, &mock),
            token("user-1"),
            "chapter-1".into(),
            instr
        )
        .await
        .is_err()
    );
}

// composite_archive(archive)(positive,negative): composites without translation Pages are captured and cleaned atomically; failed archive persistence retains images and issues.
#[tokio::test]
async fn composite_archive_without_source_pages_is_atomic() {
    for fails in [false, true] {
        let mut mock = seed(RoleField::ADMIN);

        if fails {
            mock = mock.with_archive_commit_failure();
        }

        mock.seed_assignment(assignment(
            "chapter-1",
            "user-1",
            RoleMask::from(RoleField::REVIEWER),
        ));

        let allocated = alloc(&mock, vec![page(None, 1)]).await.unwrap();

        let page = allocated.pages.first().unwrap();

        mark(&mock, &page.page_artwork_id, page.image_ver)
            .await
            .unwrap();

        review(&mock, &page.page_artwork_id).await;

        {
            let mut state = mock.state.lock().unwrap();

            state.assignments.clear();

            let chapter_info = state.chapters.first_mut().unwrap();

            chapter_info.stages = chapter_info
                .stages
                .try_set_phase(Stage::Publish, StagePhase::Completed)
                .unwrap();

            drop(state);
        }

        let before = mock.snapshot();

        assert!(before.pages.is_empty());

        let result = comic_archive_usecase::archive(
            (&mock, &mock, &mock),
            token("user-1"),
            "comic-1".into(),
        )
        .await;

        let after = mock.snapshot();

        let images = after.objs.get("page_artwork_image").unwrap();

        if fails {
            assert!(result.is_err());

            assert_eq!(after.page_artworks, before.page_artworks);

            assert_eq!(after.issues, before.issues);

            assert!(images.get(&page.page_artwork_id).unwrap().meta.is_some());

            continue;
        }

        result.unwrap();

        assert!(after.page_artworks.is_empty());

        assert!(after.issues.is_empty());

        assert!(images.is_empty());

        let payload: serde_json::Value = serde_json::from_str(
            &after.comic_archives.first().unwrap().archived_payload,
        )
        .unwrap();

        let archived = payload.pointer("/chapters/0/page_artworks/0").unwrap();

        assert_eq!(
            archived.get("source_page_artwork_id").unwrap(),
            &page.page_artwork_id
        );

        assert_eq!(
            archived.pointer("/issues/0/note").unwrap(),
            "fix this text"
        );
    }
}
