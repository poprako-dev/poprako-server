use super::*;

use std::collections::HashMap;

use poprako_util::i18n::trl_kv;

use crate::data::instr::unit::SearchChapterUnitInfosInstr;
use crate::model::read::proj::unit::UnitInfo;
use crate::model::shared::unit::UnitCoord;

#[tokio::test]
async fn single_character_search_preserves_page_and_unit_order() {
    //
    let mock = search_scope(2);

    mock.seed_unit(unit_info("unit-b", "page-1", None, "日 second"));

    mock.seed_unit(unit_info("unit-a", "page-1", Some("unit-b"), "first 日"));

    mock.seed_unit(unit_info("unit-c", "page-2", None, "third 日"));

    let found_infos =
        search_infos((&mock, &mock), token("translator-1"), search_instr("日"))
            .await
            .unwrap();

    assert_eq!(
        found_infos
            .into_iter()
            .map(|unit_info| unit_info.id)
            .collect::<Vec<_>>(),
        ["unit-a", "unit-b", "unit-c"]
    );
}

#[tokio::test]
async fn exactly_one_hundred_matches_succeed() {
    //
    let mock = search_scope(1);

    seed_matching_chain(&mock, "page-1", 100);

    let found_infos =
        search_infos((&mock, &mock), token("translator-1"), search_instr("日"))
            .await
            .unwrap();

    assert_eq!(found_infos.len(), 100);

    assert_eq!(found_infos[0].id, "page-1-unit-000");

    assert_eq!(found_infos[99].id, "page-1-unit-099");
}

#[tokio::test]
async fn one_hundred_and_first_match_returns_args_message() {
    //
    let mock = search_scope(2);

    seed_matching_chain(&mock, "page-1", 100);

    seed_matching_chain(&mock, "page-2", 1);

    let error =
        search_infos((&mock, &mock), token("translator-1"), search_instr("日"))
            .await
            .unwrap_err();

    let args = HashMap::from([(
        "match_limit".into(),
        MAX_UNIT_SEARCH_MATCH_COUNT.into(),
    )]);

    let expected_message = trl_kv("error-unit-search-too-many-matches", &args);

    assert!(matches!(
        error,
        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        } if message == expected_message
    ));
}

#[tokio::test]
async fn excess_in_first_batch_does_not_read_later_page_batch() {
    //
    let mock = search_scope(21);

    seed_matching_chain(&mock, "page-1", 100);

    seed_matching_chain(&mock, "page-2", 1);

    mock.seed_unit(unit_info("broken-a", "page-21", None, "sentinel"));

    mock.seed_unit(unit_info("broken-b", "page-21", None, "sentinel"));

    let error =
        search_infos((&mock, &mock), token("translator-1"), search_instr("日"))
            .await
            .unwrap_err();

    let args = HashMap::from([(
        "match_limit".into(),
        MAX_UNIT_SEARCH_MATCH_COUNT.into(),
    )]);

    let expected_message = trl_kv("error-unit-search-too-many-matches", &args);

    assert!(matches!(
        error,
        BaseError::Expected {
            variant: ExpectedVariant::Args,
            message,
        } if message == expected_message
    ));
}

#[tokio::test]
async fn search_is_literal_case_sensitive_and_field_specific() {
    let mock = search_scope(1);

    let mut unit_info =
        unit_info("unit-a", "page-1", None, r"Case-sensitive %_\\ marker");

    unit_info.proofread_text = Some("proofread marker".to_string());

    mock.seed_unit(unit_info);

    let literal_infos = search_infos(
        (&mock, &mock),
        token("translator-1"),
        search_instr(r"%_\\"),
    )
    .await
    .unwrap();

    assert_eq!(literal_infos.len(), 1);

    let wrong_case_infos = search_infos(
        (&mock, &mock),
        token("translator-1"),
        search_instr("case-sensitive"),
    )
    .await
    .unwrap();

    assert!(wrong_case_infos.is_empty());

    let wrong_field_infos = search_infos(
        (&mock, &mock),
        token("translator-1"),
        search_instr("proofread marker"),
    )
    .await
    .unwrap();

    assert!(wrong_field_infos.is_empty());

    let proofread_infos = search_infos(
        (&mock, &mock),
        token("translator-1"),
        search_instr_for(UnitTextPart::ProofreadText, "proofread marker"),
    )
    .await
    .unwrap();

    assert_eq!(proofread_infos[0].id, "unit-a");
}

#[tokio::test]
async fn hidden_link_is_excluded_without_changing_visible_order() {
    let mock = search_scope(1);

    mock.seed_unit(unit_info("unit-b", "page-1", None, "match"));

    let mut hidden_info =
        unit_info("unit-hidden", "page-1", Some("unit-b"), "match");

    hidden_info.hidden_at = Some(OffsetDateTime::now_utc());

    mock.seed_unit(hidden_info);

    mock.seed_unit(unit_info("unit-a", "page-1", Some("unit-hidden"), "match"));

    let found_infos = search_infos(
        (&mock, &mock),
        token("translator-1"),
        search_instr("match"),
    )
    .await
    .unwrap();

    assert_eq!(
        found_infos
            .into_iter()
            .map(|unit_info| unit_info.id)
            .collect::<Vec<_>>(),
        ["unit-a", "unit-b"]
    );
}

#[tokio::test]
async fn nul_phrase_returns_no_match_after_authorization() {
    let mock = search_scope(1);

    mock.seed_unit(unit_info("unit-a", "page-1", None, "match"));

    let found_infos =
        search_infos((&mock, &mock), token("translator-1"), search_instr("\0"))
            .await
            .unwrap();

    assert!(found_infos.is_empty());
}

// Build a Chapter search instruction selecting translated text.
fn search_instr(phrase: &str) -> SearchChapterUnitInfosInstr {
    search_instr_for(UnitTextPart::TranslatedText, phrase)
}

// Build a Chapter search instruction selecting one text field.
fn search_instr_for(
    part: UnitTextPart,
    phrase: &str,
) -> SearchChapterUnitInfosInstr {
    SearchChapterUnitInfosInstr {
        chapter_id: "chapter-1".to_string(),
        part,
        phrase: phrase.to_string(),
    }
}

// Build the standard Unit scope with the requested number of ordered Pages.
fn search_scope(page_count: usize) -> Mock {
    //
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    for page_index in 1..page_count {
        //
        let mut page_info = page();

        page_info.id = format!("page-{}", page_index + 1);

        page_info.index = page_index;

        mock.seed_page(page_info);
    }

    mock
}

// Seed one valid matching linked Unit chain in storage order.
fn seed_matching_chain(mock: &Mock, page_id: &str, unit_count: usize) {
    //
    for unit_index in 0..unit_count {
        //
        let unit_id = format!("{}-unit-{:03}", page_id, unit_index);

        let next_id = (unit_index + 1 < unit_count)
            .then(|| format!("{}-unit-{:03}", page_id, unit_index + 1));

        let unit_info = unit_info(&unit_id, page_id, next_id.as_deref(), "日");

        mock.seed_unit(unit_info);
    }
}

// Build one visible persisted Unit fixture.
fn unit_info(
    id: &str,
    page_id: &str,
    next_id: Option<&str>,
    translated_text: &str,
) -> UnitInfo {
    //
    let current_time = OffsetDateTime::now_utc();

    UnitInfo {
        id: id.to_string(),
        page_id: page_id.to_string(),
        next_id: next_id.map(str::to_string),
        is_bubble: true,
        coord: UnitCoord {
            x_coord: 1.0,
            y_coord: 2.0,
        },
        translated_text: Some(translated_text.to_string()),
        last_translator_id: Some("translator-1".to_string()),
        is_proofread: false,
        proofread_text: None,
        last_proofreader_id: None,
        hidden_at: None,
        created_at: current_time,
        updated_at: current_time,
    }
}
