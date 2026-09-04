use super::*;

use crate::data::instr::page::ListEdittedDiffPageIdsInstr;
use crate::model::read::proj::unit::UnitInfo;
use crate::model::shared::unit::UnitCoord;
use crate::result::{BaseError, ExpectedVariant};
use crate::usecase::page::list::list_editted_diff_page_ids;
use crate::value::page::MAX_CHAPTER_PAGE_COUNT;
use crate::value::role::RoleField;

#[tokio::test]
async fn list_page_ids_filters_diffs_and_preserves_page_order() {
    let mock = diff_scope();

    mock.seed_unit(unit_info(
        "unit-page-1-a",
        "page-1",
        Some("translated"),
        Some("proofread"),
        false,
        false,
    ));

    mock.seed_unit(unit_info(
        "unit-page-1-b",
        "page-1",
        Some("another translation"),
        Some("another proofread"),
        true,
        false,
    ));

    mock.seed_unit(unit_info(
        "unit-page-2-equal",
        "page-2",
        Some("same"),
        Some("same"),
        true,
        false,
    ));

    mock.seed_unit(unit_info(
        "unit-page-2-empty",
        "page-2",
        None,
        Some(" \t\r\n\u{3000}"),
        true,
        false,
    ));

    mock.seed_unit(unit_info(
        "unit-page-2-hidden",
        "page-2",
        Some("translated"),
        Some("hidden proofread"),
        true,
        true,
    ));

    mock.seed_unit(unit_info(
        "unit-page-3",
        "page-3",
        None,
        Some("proofread without translation"),
        true,
        false,
    ));

    let val =
        list_editted_diff_page_ids((&mock,), page_token("user-1"), instr())
            .await
            .unwrap();

    assert_eq!(val.page_ids, ["page-3", "page-1"]);
}

#[tokio::test]
async fn list_page_ids_returns_empty_when_no_visible_diff_exists() {
    let mock = read_scope();

    mock.seed_unit(unit_info(
        "unit-1",
        "page-1",
        Some("same"),
        Some("same"),
        true,
        false,
    ));

    let val =
        list_editted_diff_page_ids((&mock,), page_token("user-1"), instr())
            .await
            .unwrap();

    assert!(val.page_ids.is_empty());
}

#[tokio::test]
async fn list_page_ids_accepts_team_member_without_chapter_assignment() {
    let mock = read_scope();

    mock.seed_member(page_member(
        "member-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let val =
        list_editted_diff_page_ids((&mock,), page_token("member-1"), instr())
            .await
            .unwrap();

    assert!(val.page_ids.is_empty());
}

#[tokio::test]
async fn list_page_ids_rejects_user_without_chapter_access() {
    let mock = read_scope();

    let error =
        list_editted_diff_page_ids((&mock,), page_token("outsider"), instr())
            .await
            .unwrap_err();

    assert!(matches!(
        error,
        BaseError::Expected {
            variant: ExpectedVariant::Perm,
            ..
        }
    ));
}

#[tokio::test]
async fn list_page_ids_rejects_excess_pages_even_without_matching_diffs() {
    let mock = Mock::new();

    seed_page_scope(&mock, MAX_CHAPTER_PAGE_COUNT + 1);

    mock.seed_assignment(page_assignment(
        "user-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    for index in 0..=MAX_CHAPTER_PAGE_COUNT {
        mock.seed_page(page_model(&format!("page-{index:03}"), index));
    }

    let error =
        list_editted_diff_page_ids((&mock,), page_token("user-1"), instr())
            .await
            .unwrap_err();

    assert!(matches!(error, BaseError::Unrecoverable { .. }));
}

// Build one authorized Page read scope.
fn read_scope() -> Mock {
    let mock = Mock::new();

    seed_page_scope(&mock, 1);

    mock.seed_page(page_model("page-1", 0));

    mock.seed_assignment(page_assignment(
        "user-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock
}

// Build a Chapter scope whose Pages are deliberately seeded out of order.
fn diff_scope() -> Mock {
    let mock = read_scope();

    let mut state = mock.state.lock().unwrap();

    state.pages.clear();

    state.pages.extend([
        page_model("page-1", 2),
        page_model("page-2", 1),
        page_model("page-3", 0),
    ]);

    drop(state);

    mock
}

// Build the fixed request for the default Chapter fixture.
fn instr() -> ListEdittedDiffPageIdsInstr {
    ListEdittedDiffPageIdsInstr {
        chapter_id: "chapter-1".to_string(),
    }
}

// Build one Unit fixture with independently controlled text and visibility.
fn unit_info(
    id: &str,
    page_id: &str,
    translated_text: Option<&str>,
    proofread_text: Option<&str>,
    is_proofread: bool,
    is_hidden: bool,
) -> UnitInfo {
    let current_time = OffsetDateTime::now_utc();

    UnitInfo {
        id: id.to_string(),

        page_id: page_id.to_string(),
        next_id: None,

        is_bubble: true,

        coord: UnitCoord {
            x_coord: 1.0,
            y_coord: 2.0,
        },

        translated_text: translated_text.map(str::to_string),
        last_translator_id: None,

        is_proofread,
        proofread_text: proofread_text.map(str::to_string),
        last_proofreader_id: None,

        hidden_at: is_hidden.then_some(current_time),

        created_at: current_time,
        updated_at: current_time,
    }
}
