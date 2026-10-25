use super::*;

use crate::data::instr::page_source::{
    ListPageSourceUnitDiffStatsInstr, ListPageSourceUnitFlaggedStatsInstr,
};
use crate::model::read::proj::unit::UnitInfo;
use crate::model::shared::unit::UnitCoord;
use crate::result::{BaseError, ExpectedVariant};
use crate::usecase::page_source::list::{
    list_unit_diff_stats, list_unit_flagged_stats,
};
use crate::value::page_source::MAX_CHAPTER_PAGE_SOURCE_COUNT;
use crate::value::role::RoleField;

#[tokio::test]
async fn list_unit_diff_stats_filters_diffs_and_preserves_page_source_order() {
    let mock = diff_scope();

    mock.seed_unit(unit_info(
        "unit--09338981995",
        "page_source--00000000001",
        (Some("translated"), Some("proofread")),
        false,
        false,
    ));

    mock.seed_unit(unit_info(
        "unit--75527387370",
        "page_source--00000000001",
        (Some("another translation"), Some("another proofread")),
        true,
        false,
    ));

    mock.seed_unit(unit_info(
        "unit--87943331855",
        "page_source--00000000002",
        (Some("same"), Some("same")),
        true,
        false,
    ));

    mock.seed_unit(unit_info(
        "unit--28090898527",
        "page_source--00000000002",
        (None, Some(" \t\r\n\u{3000}")),
        true,
        false,
    ));

    mock.seed_unit(unit_info(
        "unit--44500282604",
        "page_source--00000000002",
        (Some("translated"), Some("hidden proofread")),
        true,
        true,
    ));

    mock.seed_unit(unit_info(
        "unit--66669095085",
        "page_source--00000000003",
        (None, Some("proofread without translation")),
        true,
        false,
    ));

    let val = list_unit_diff_stats(
        (&mock,),
        page_source_token("user--00000000001"),
        instr(),
    )
    .await
    .unwrap();

    assert_eq!(
        serde_json::to_value(val).unwrap(),
        serde_json::json!([
            {"page_source_id": "page_source--00000000003", "index": 0,
             "translated_unit_count": 0,
             "editted_unit_count": 0, "proofreader_append_unit_count": 1},
            {"page_source_id": "page_source--00000000001", "index": 2,
             "translated_unit_count": 2,
             "editted_unit_count": 2, "proofreader_append_unit_count": 0},
        ]),
    );
}

#[tokio::test]
async fn list_unit_diff_stats_returns_empty_when_no_visible_diff_exists() {
    let mock = read_scope();

    mock.seed_unit(unit_info(
        "unit--00000000001",
        "page_source--00000000001",
        (Some("same"), Some("same")),
        true,
        false,
    ));

    let val = list_unit_diff_stats(
        (&mock,),
        page_source_token("user--00000000001"),
        instr(),
    )
    .await
    .unwrap();

    assert!(val.is_empty());
}

#[tokio::test]
async fn list_unit_diff_stats_accepts_team_member_without_chapter_assignment() {
    let mock = read_scope();

    mock.seed_member(page_source_member(
        "member--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let val = list_unit_diff_stats(
        (&mock,),
        page_source_token("member--00000000001"),
        instr(),
    )
    .await
    .unwrap();

    assert!(val.is_empty());
}

#[tokio::test]
async fn list_unit_diff_stats_rejects_user_without_chapter_access() {
    let mock = read_scope();

    let error =
        list_unit_diff_stats((&mock,), page_source_token("outsider"), instr())
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
async fn list_unit_diff_stats_rejects_excess_page_sources_even_without_matching_diffs()
 {
    let mock = Mock::new();

    seed_page_source_scope(&mock, MAX_CHAPTER_PAGE_SOURCE_COUNT + 1);

    mock.seed_assignment(page_source_assignment(
        "user--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    for index in 0..=MAX_CHAPTER_PAGE_SOURCE_COUNT {
        mock.seed_page(page_source_model(&format!("page-{index:03}"), index));
    }

    let error = list_unit_diff_stats(
        (&mock,),
        page_source_token("user--00000000001"),
        instr(),
    )
    .await
    .unwrap_err();

    assert!(matches!(error, BaseError::Unrecoverable { .. }));
}

#[tokio::test]
async fn list_unit_diff_stats_counts_text_states_independently_of_approval() {
    let mock = read_scope();

    let cases = [
        (Some("translated"), None),
        (Some("same"), Some("same")),
        (Some("translated"), Some("revised")),
        (Some("A"), Some("a")),
        (Some("text"), Some(" text ")),
        (Some("translated"), Some("")),
        (Some("translated"), Some(" \t\u{3000}")),
        (None, Some("appended")),
        (Some(""), Some("appended")),
        (Some(" \t\r\n"), Some("appended")),
        (Some("\u{0085}\u{00a0}\u{3000}"), Some("appended")),
        (None, None),
        (Some(" "), Some("\u{3000}")),
    ];

    for (index, (translation, revision)) in cases.into_iter().enumerate() {
        mock.seed_unit(unit_info(
            &format!("unit-{index}"),
            "page_source--00000000001",
            (translation, revision),
            index % 2 == 0,
            false,
        ));

        mock.seed_unit(unit_info(
            &format!("hidden-{index}"),
            "page_source--00000000001",
            (translation, revision),
            index % 2 != 0,
            true,
        ));
    }

    let mut other_page = page_source_model("other-page", 0);

    other_page.chapter_id = "other-chapter".into();

    mock.seed_page(other_page);

    mock.seed_unit(unit_info(
        "other-unit",
        "other-page",
        (Some("x"), Some("y")),
        true,
        false,
    ));

    let stats = list_unit_diff_stats(
        (&mock,),
        page_source_token("user--00000000001"),
        instr(),
    )
    .await
    .unwrap();

    assert_eq!(stats.len(), 1);

    assert_eq!(stats.first().unwrap().translated_unit_count, 7);

    assert_eq!(stats.first().unwrap().editted_unit_count, 3);

    assert_eq!(stats.first().unwrap().proofreader_append_unit_count, 4);
}

#[tokio::test]
async fn list_unit_diff_stats_accepts_exact_page_source_limit() {
    let mock = read_scope();

    for index in 1..MAX_CHAPTER_PAGE_SOURCE_COUNT {
        mock.seed_page(page_source_model(
            &format!("page-limit-{index}"),
            index,
        ));
    }

    let stats = list_unit_diff_stats(
        (&mock,),
        page_source_token("user--00000000001"),
        instr(),
    )
    .await
    .unwrap();

    assert!(stats.is_empty());
}

#[tokio::test]
async fn list_unit_diff_stats_rejects_missing_chapter() {
    let mock = read_scope();

    let request = ListPageSourceUnitDiffStatsInstr {
        chapter_id: "missing-chapter".into(),
    };

    let error = list_unit_diff_stats(
        (&mock,),
        page_source_token("user--00000000001"),
        request,
    )
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        }
    ));
}

// Build one authorized PageSource read scope.
fn read_scope() -> Mock {
    let mock = Mock::new();

    seed_page_source_scope(&mock, 1);

    mock.seed_page(page_source_model("page_source--00000000001", 0));

    mock.seed_assignment(page_source_assignment(
        "user--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    mock
}

// Build a Chapter scope whose PageSources are deliberately seeded out of order.
fn diff_scope() -> Mock {
    let mock = read_scope();

    let mut state = mock.state.lock().unwrap();

    state.pages.clear();

    state.pages.extend([
        page_source_model("page_source--00000000001", 2),
        page_source_model("page_source--00000000002", 1),
        page_source_model("page_source--00000000003", 0),
    ]);

    drop(state);

    mock
}

// Build the fixed request for the default Chapter fixture.
fn instr() -> ListPageSourceUnitDiffStatsInstr {
    ListPageSourceUnitDiffStatsInstr {
        chapter_id: "chapter--00000000001".to_string(),
    }
}

// Build one Unit fixture with independently controlled text and visibility.
fn unit_info(
    id: &str,
    page_source_id: &str,
    (translated_text, proofread_text): (Option<&str>, Option<&str>),
    is_proofread: bool,
    is_hidden: bool,
) -> UnitInfo {
    let current_time = OffsetDateTime::now_utc();

    UnitInfo {
        id: id.to_string(),

        page_source_id: page_source_id.to_string(),
        next_id: None,

        is_bubble: true,
        is_flagged: false,

        coord: UnitCoord {
            x_coord: 1.0,
            y_coord: 2.0,
        },

        typography_note: None,
        translation_note: None,
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

// list_unit_flagged_stats(flagged)(positive): filters hidden Units and preserves original PageSource order.
#[tokio::test]
async fn flagged_stats_count_visible_units_in_page_source_order() {
    let mock = diff_scope();

    for (id, page_source_id, flagged, hidden) in [
        ("a", "page_source--00000000001", true, false),
        ("b", "page_source--00000000001", true, false),
        ("c", "page_source--00000000001", true, true),
        ("d", "page_source--00000000002", false, false),
        ("e", "page_source--00000000003", true, false),
        ("f", "unrelated-page", true, false),
    ] {
        let mut unit =
            unit_info(id, page_source_id, (None, None), false, hidden);

        unit.is_flagged = flagged;

        mock.seed_unit(unit);
    }

    let stats = list_unit_flagged_stats(
        (&mock,),
        page_source_token("user--00000000001"),
        ListPageSourceUnitFlaggedStatsInstr {
            chapter_id: "chapter--00000000001".into(),
        },
    )
    .await
    .unwrap();

    assert_eq!(
        serde_json::to_value(stats).unwrap(),
        serde_json::json!([
            {"page_source_id":"page_source--00000000003","index":0,
             "flagged_unit_count":1},
            {"page_source_id":"page_source--00000000001","index":2,
             "flagged_unit_count":2},
        ])
    );
}

// list_unit_flagged_stats(flagged)(negative): access and whole-Chapter page bounds match other PageSource reads.
#[tokio::test]
async fn flagged_stats_enforce_access_and_page_source_limit_without_matches() {
    let mock = read_scope();

    mock.seed_member(page_source_member(
        "member--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let request = || ListPageSourceUnitFlaggedStatsInstr {
        chapter_id: "chapter--00000000001".into(),
    };

    assert!(
        list_unit_flagged_stats(
            (&mock,),
            page_source_token("member--00000000001"),
            request()
        )
        .await
        .unwrap()
        .is_empty()
    );

    let denied = list_unit_flagged_stats(
        (&mock,),
        page_source_token("outsider"),
        request(),
    )
    .await;

    assert!(matches!(
        denied,
        Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            ..
        })
    ));

    let missing = list_unit_flagged_stats(
        (&mock,),
        page_source_token("user--00000000001"),
        ListPageSourceUnitFlaggedStatsInstr {
            chapter_id: "missing".into(),
        },
    )
    .await;

    assert!(matches!(
        missing,
        Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        })
    ));

    for index in 1..MAX_CHAPTER_PAGE_SOURCE_COUNT {
        mock.seed_page(page_source_model(&format!("extra-{index}"), index));
    }

    assert!(
        list_unit_flagged_stats(
            (&mock,),
            page_source_token("user--00000000001"),
            request()
        )
        .await
        .unwrap()
        .is_empty()
    );

    mock.seed_page(page_source_model("excess", MAX_CHAPTER_PAGE_SOURCE_COUNT));

    assert!(matches!(
        list_unit_flagged_stats(
            (&mock,),
            page_source_token("user--00000000001"),
            request()
        )
        .await,
        Err(BaseError::Unrecoverable { .. })
    ));
}
