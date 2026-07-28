// import(import)(positive): proofreader imports real LabelPlus material transactionally.
// import(import)(negative): page-count mismatch rejects import and leaves units and counters unchanged.

use super::*;
use crate::data::instr::chapter_port::ImportChapterTranslationInstr;

use time::OffsetDateTime;

use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::UnitInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::shared::unit::UnitCoord;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::value::chapter::StageMask;
use crate::value::chapter_port::TranslationFormat;
use crate::value::image::{ImageExt, ImageHash};
use crate::value::role::{RoleField, RoleMask};

// LabelPlus fixture content used for chapter import integration tests.
const LABEL_PLUS_MATERIAL: &str =
    include_str!("../../../../tests/materials/translations.lp.txt");

// Build a token fixture for chapter import authorization.
fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

// Build comic fixture referenced by imported chapter.
fn comic(id: &str) -> ComicInfo {
    //
    // Compose a stable comic fixture used by chapter/import permission checks.
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: "workset-1".into(),
        index: 0,
        title: "Pop Comic".into(),
        author: "author".into(),
        description: None,
        cover_key: None,
        is_cover_uploaded: false,
        cover_version: 0,
        cover_hash: ImageHash::default(),
        cover_ext: ImageExt::Png,
        chapter_count: 1,
        creator_id: "user-1".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

// Build workset fixture for chapter import scenarios.
fn workset(id: &str) -> WorksetInfo {
    //
    // Compose a stable workset fixture for assignment and chapter binding.
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

// Build chapter fixture with provided unit and proofread counters.
fn chapter(
    page_count: i32,
    total_unit_count: i32,
    proofread_unit_count: i32,
) -> ChapterInfo {
    //
    // Build a chapter fixture with configurable unit counters.
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: "chapter-1".into(),
        comic_id: "comic-1".into(),
        is_pinned: true,
        index: 3,
        subtitle: "Arrival".into(),
        page_count,
        total_unit_count,
        translated_unit_count: total_unit_count,
        proofread_unit_count,
        stages: StageMask::try_from(0u32).ok().unwrap(),
        creator_id: "user-1".into(),
        comic: None,
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

// Build assignment fixture for import permission checks.
fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> AssignmentInfo {
    //
    // Create a member assignment record for import permission scenarios.
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

// Build page fixture and image metadata for import material.
fn page(
    id: &str,
    index: i32,
    total_unit_count: i32,
    proofread_unit_count: i32,
) -> PageInfo {
    //
    // Build one page fixture and pre-seed image metadata.
    let time = OffsetDateTime::now_utc();

    PageInfo {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        index,
        image_key: Some(format!("page-{}.png", index)),
        is_image_uploaded: true,
        image_version: 1,
        image_hash: ImageHash::new([0u8; 32]),
        image_ext: ImageExt::Png,
        total_unit_count,
        translated_unit_count: total_unit_count,
        proofread_unit_count,
        created_at: time,
        updated_at: time,
    }
}

// Build unit fixture with translator metadata and optional translated content.
fn unit(id: &str, page_id: &str, _index: i32, text: &str) -> UnitInfo {
    //
    let time = OffsetDateTime::now_utc();

    UnitInfo {
        id: id.into(),
        page_id: page_id.into(),
        next_id: None,
        is_bubble: true,
        is_proofread: false,
        coord: UnitCoord {
            x_coord: 0.25,
            y_coord: 0.75,
        },
        translated_text: Some(text.into()),
        last_translator_id: Some("translator-1".into()),
        proofread_text: None,
        last_proofreader_id: None,
        hidden_at: None,
        created_at: time,
        updated_at: time,
    }
}

// Seed workset/comic/chapter/assignment baseline for chapter import.
fn seed_base(
    mock: &Mock,
    page_count: i32,
    total_unit_count: i32,
    proofread_unit_count: i32,
) {
    //
    // Seed the minimal base graph (workset, comic, chapter, assignment).
    mock.seed_workset(workset("workset-1"));

    mock.seed_comic(comic("comic-1"));

    mock.seed_chapter(chapter(
        page_count,
        total_unit_count,
        proofread_unit_count,
    ));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::PROOFREADER),
    ));
}

// Seed a full set of placeholder pages used by import material input.
fn seed_material_pages(mock: &Mock) {
    for index in 0..9 {
        mock.seed_page(page(&format!("page-{}", index + 1), index, 0, 0));
    }
}

#[tokio::test]
async fn import_label_plus_material_updates_units_and_counters() {
    //
    // Verify import applies unit text/proofread content and updates chapter counters.
    let mock = Mock::new();

    seed_base(&mock, 9, 0, 0);

    seed_material_pages(&mock);

    let imported = import(
        (&mock, &mock),
        token("user-1"),
        ImportChapterTranslationInstr {
            format: TranslationFormat::LabelPlus,
            content: LABEL_PLUS_MATERIAL.into(),
        },
        "chapter-1".into(),
    )
    .await;

    let imported = match imported {
        //
        // Convert transport errors into explicit panics in this happy-path unit test.
        Ok(imported) => imported,

        Err(_) => panic!("expected import success"),
    };

    let snapshot = mock.snapshot();

    let first_unit = snapshot
        .units
        .iter()
        .find(|unit_info| unit_info.page_id == "page-1")
        .unwrap();

    let last_unit = snapshot
        .units
        .iter()
        .filter(|unit_info| unit_info.page_id == "page-9")
        .last()
        .unwrap();

    let chapter_info = snapshot
        .chapters
        .iter()
        .find(|chapter_info| chapter_info.id == "chapter-1")
        .unwrap();

    assert_eq!(imported.imported_page_count, 9);

    assert_eq!(imported.imported_unit_count, 65);

    assert_eq!(snapshot.units.len(), 65);

    assert_eq!(first_unit.proofread_text, Some("喂 游斗哥".into()));

    assert_eq!(first_unit.last_proofreader_id, Some("user-1".into()));

    assert_eq!(
        last_unit.proofread_text,
        Some("哥哥对次女可爱的\n小心思毫无察觉".into())
    );

    assert_eq!(chapter_info.total_unit_count, 65);

    assert_eq!(chapter_info.proofread_unit_count, 65);
}

#[tokio::test]
async fn import_rejects_page_count_mismatch_without_mutation() {
    //
    // Verify mismatched page counts fail atomically and avoid mutating seeded data.
    let mock = Mock::new();

    seed_base(&mock, 2, 1, 0);

    mock.seed_page(page("page-1", 0, 1, 0));

    mock.seed_page(page("page-2", 1, 0, 0));

    mock.seed_unit(unit("unit-a", "page-1", 0, "old"));

    let err = import(
        (&mock, &mock),
        token("user-1"),
        ImportChapterTranslationInstr {
            format: TranslationFormat::LabelPlus,
            content: LABEL_PLUS_MATERIAL.into(),
        },
        "chapter-1".into(),
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert_eq!(snapshot.units.len(), 1);

    assert_eq!(snapshot.units[0].translated_text, Some("old".into()));

    assert_eq!(snapshot.chapters[0].total_unit_count, 1);
}
