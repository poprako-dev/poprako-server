// save_edits(save_edits)(positive): token identity is persisted and save returns Unit.
// save_edits(save_edits)(positive): Delete tombstones and Patch restores a Unit.
// save_edits(save_edits)(positive): concurrent inserts before one anchor remain a complete chain.
// save_edits(save_edits)(negative): translator revision edits are rejected and rolled back.

// Unit search tests with dedicated high-cardinality fixtures.
mod search;

use super::transform::transform;
use super::*;

use time::OffsetDateTime;

use crate::data::instr::unit::{
    ListPageUnitInfosInstr, SavePageUnitEditsInstr, TransformChapterUnitsInstr,
    UnitCoordInstr, UnitEditInstr, UnitRevisionInstr, UnitTextTransformInstr,
    UnitTransformInstr, UnitTranslationInstr,
};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{BaseError, ExpectedVariant};
use crate::util::Patch;
use crate::value::chapter::mask::StageMask;
use crate::value::role::{RoleField, RoleMask};
use crate::value::unit::UnitTextPart;

#[tokio::test]
async fn create_uses_token_identity_and_updates_counters() {
    //
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![create("local-1", None, Some("translated"))]),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.units.len(), 1);

    assert_eq!(
        snapshot.units[0].last_translator_id.as_deref(),
        Some("translator-1")
    );

    assert_eq!(snapshot.pages[0].total_unit_count, 1);

    assert_eq!(snapshot.pages[0].translated_unit_count, 1);

    assert_eq!(snapshot.chapters[0].total_unit_count, 1);
}

#[tokio::test]
async fn transform_updates_selected_text_without_cascading_targets() {
    //
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let mut transform_assignment =
        assignment(RoleMask::from(RoleField::TRANSLATOR));

    transform_assignment.id = "assignment-2".to_string();

    transform_assignment.user_id = "translator-2".to_string();

    mock.seed_assignment(transform_assignment);

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![create("local-1", None, Some("alpha beta"))]),
    )
    .await
    .unwrap();

    let unit_id = mock.snapshot().units[0].id.clone();

    transform(
        (&mock, &mock),
        token("translator-2"),
        "chapter-1".to_string(),
        TransformChapterUnitsInstr {
            part: UnitTextPart::TranslatedText,
            units: vec![UnitTransformInstr {
                unit_id,
                transforms: vec![
                    UnitTextTransformInstr {
                        origin: "alpha".to_string(),
                        target: "beta".to_string(),
                    },
                    UnitTextTransformInstr {
                        origin: "beta".to_string(),
                        target: "final".to_string(),
                    },
                ],
            }],
        },
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(
        snapshot.units[0].translated_text.as_deref(),
        Some("beta final")
    );

    assert_eq!(
        snapshot.units[0].last_translator_id.as_deref(),
        Some("translator-2")
    );

    assert_eq!(snapshot.pages[0].translated_unit_count, 1);
}

#[tokio::test]
async fn overlapping_transform_rolls_back_the_complete_request() {
    //
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![create("local-1", None, Some("abcd"))]),
    )
    .await
    .unwrap();

    let before = mock.snapshot();

    let error = transform(
        (&mock, &mock),
        token("translator-1"),
        "chapter-1".to_string(),
        TransformChapterUnitsInstr {
            part: UnitTextPart::TranslatedText,
            units: vec![UnitTransformInstr {
                unit_id: before.units[0].id.clone(),
                transforms: vec![
                    UnitTextTransformInstr {
                        origin: "abc".to_string(),
                        target: "first".to_string(),
                    },
                    UnitTextTransformInstr {
                        origin: "bcd".to_string(),
                        target: "second".to_string(),
                    },
                ],
            }],
        },
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

    assert_eq!(
        mock.snapshot().units[0].translated_text,
        before.units[0].translated_text
    );
}

#[tokio::test]
async fn delete_then_patch_restores_the_tombstone() {
    //
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![create("local-1", None, Some("text"))]),
    )
    .await
    .unwrap();

    let unit_id = mock.snapshot().units[0].id.clone();

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![UnitEditInstr::Delete {
            id: unit_id.clone(),
        }]),
    )
    .await
    .unwrap();

    assert!(mock.snapshot().units[0].hidden_at.is_some());

    assert_eq!(mock.snapshot().pages[0].total_unit_count, 0);

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![UnitEditInstr::Patch {
            id: unit_id,
            next_id: Patch::Skip,
            is_bubble: Some(false),
            coord: None,
            translation: Patch::Skip,
            revision: Patch::Skip,
        }]),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.units[0].hidden_at.is_none());

    assert!(!snapshot.units[0].is_bubble);

    assert_eq!(snapshot.pages[0].total_unit_count, 1);
}

#[tokio::test]
async fn translator_revision_edit_is_rejected_without_mutation() {
    //
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![create("local-1", None, Some("text"))]),
    )
    .await
    .unwrap();

    let before = mock.snapshot();

    let error = save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![UnitEditInstr::Patch {
            id: before.units[0].id.clone(),
            next_id: Patch::Skip,
            is_bubble: None,
            coord: None,
            translation: Patch::Skip,
            revision: Patch::Assign {
                value: UnitRevisionInstr {
                    is_proofread: true,
                    proofread_text: Some("proofread".to_string()),
                },
            },
        }]),
    )
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        BaseError::Expected {
            variant: ExpectedVariant::Perm,
            ..
        }
    ));

    let after = mock.snapshot();

    assert_eq!(
        after.units[0].proofread_text,
        before.units[0].proofread_text
    );

    assert_eq!(after.pages[0].proofread_unit_count, 0);
}

#[tokio::test]
async fn proofreader_and_dual_role_apply_only_their_allowed_fields() {
    //
    let proofreader = save_scope(RoleMask::from(RoleField::PROOFREADER));

    save_edits(
        (&proofreader, &proofreader),
        token("translator-1"),
        save_instr(vec![UnitEditInstr::Create {
            local_id: "proofreader-local".to_string(),
            next_id: None,
            is_bubble: true,
            coord: UnitCoordInstr {
                x_coord: 1.0,
                y_coord: 2.0,
            },
            translation: None,
            revision: Some(UnitRevisionInstr {
                is_proofread: true,
                proofread_text: Some("proofread".to_string()),
            }),
        }]),
    )
    .await
    .unwrap();

    let proofreader_snapshot = proofreader.snapshot();

    assert_eq!(
        proofreader_snapshot.units[0].last_proofreader_id.as_deref(),
        Some("translator-1")
    );

    let dual_roles = RoleMask::from(RoleField::TRANSLATOR)
        .union(RoleMask::from(RoleField::PROOFREADER));

    let dual = save_scope(dual_roles);

    save_edits(
        (&dual, &dual),
        token("translator-1"),
        save_instr(vec![UnitEditInstr::Create {
            local_id: "dual-local".to_string(),
            next_id: None,
            is_bubble: true,
            coord: UnitCoordInstr {
                x_coord: 1.0,
                y_coord: 2.0,
            },
            translation: Some(UnitTranslationInstr {
                translated_text: "translated".to_string(),
            }),
            revision: Some(UnitRevisionInstr {
                is_proofread: true,
                proofread_text: Some("proofread".to_string()),
            }),
        }]),
    )
    .await
    .unwrap();

    let dual_snapshot = dual.snapshot();

    assert_eq!(dual_snapshot.pages[0].translated_unit_count, 1);

    assert_eq!(dual_snapshot.pages[0].proofread_unit_count, 1);
}

#[tokio::test]
async fn concurrent_same_anchor_inserts_preserve_all_nodes() {
    //
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![create("anchor-local", None, Some("anchor"))]),
    )
    .await
    .unwrap();

    let anchor_id = mock.snapshot().units[0].id.clone();

    let first_mock = mock.clone();

    let first_anchor = anchor_id.clone();

    let first = tokio::spawn(async move {
        save_edits(
            (&first_mock, &first_mock),
            token("translator-1"),
            save_instr(vec![create(
                "first-local",
                Some(first_anchor),
                Some("first"),
            )]),
        )
        .await
    });

    let second_mock = mock.clone();

    let second = tokio::spawn(async move {
        save_edits(
            (&second_mock, &second_mock),
            token("translator-1"),
            save_instr(vec![create(
                "second-local",
                Some(anchor_id),
                Some("second"),
            )]),
        )
        .await
    });

    first.await.unwrap().unwrap();

    second.await.unwrap().unwrap();

    let listed = list_infos(
        (&mock,),
        token("translator-1"),
        ListPageUnitInfosInstr {
            page_id: "page-1".to_string(),
        },
    )
    .await
    .unwrap();

    assert_eq!(listed.unit_infos.len(), 3);

    assert_eq!(
        listed.unit_infos.last().unwrap().translated_text.as_deref(),
        Some("anchor")
    );
}

// Build save request instr for a fixed page id.
fn save_instr(edits: Vec<UnitEditInstr>) -> SavePageUnitEditsInstr {
    SavePageUnitEditsInstr {
        page_id: "page-1".to_string(),
        edits,
    }
}

// Build a create-unit edit fixture for a page.
fn create(
    local_id: &str,
    next_id: Option<String>,
    translated_text: Option<&str>,
) -> UnitEditInstr {
    UnitEditInstr::Create {
        local_id: local_id.to_string(),
        next_id,
        is_bubble: true,
        coord: UnitCoordInstr {
            x_coord: 1.0,
            y_coord: 2.0,
        },
        translation: translated_text.map(|text| UnitTranslationInstr {
            translated_text: text.to_string(),
        }),
        revision: None,
    }
}

// Build token payload used by unit-edit authorization checks.
fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.to_string(),
    }
}

// Build a mock scope with workset/chapter/page/assignment fixtures for tests.
fn save_scope(roles: RoleMask) -> Mock {
    //
    // Build a minimal snapshot scope with one chapter/page/assignment setup.
    let mock = Mock::new();

    mock.seed_workset(workset());

    mock.seed_comic(comic());

    mock.seed_chapter(chapter());

    mock.seed_page(page());

    mock.seed_assignment(assignment(roles));

    mock
}

// Build baseline workset fixture consumed by other local entity builders.
fn workset() -> WorksetInfo {
    //
    // Build a baseline workset fixture.
    let current_time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: "workset-1".to_string(),
        team_id: "team-1".to_string(),
        index: 0,
        name: "workset".to_string(),
        description: None,
        comic_count: 1,
        created_at: current_time,
        updated_at: current_time,
    }
}

// Build baseline comic fixture tied to the default workset.
fn comic() -> ComicInfo {
    //
    // Build a baseline comic fixture linked to the default workset.
    let current_time = OffsetDateTime::now_utc();

    ComicInfo {
        id: "comic-1".to_string(),
        workset_id: "workset-1".to_string(),
        index: 0,
        title: "comic".to_string(),
        author: "author".to_string(),
        description: None,
        chapter_count: 1,
        creator_id: "translator-1".to_string(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: current_time,
        archived_at: None,
        created_at: current_time,
        updated_at: current_time,
    }
}

// Build baseline chapter fixture with one-page expectation.
fn chapter() -> ChapterInfo {
    //
    // Build a baseline chapter fixture for unit-level operations.
    let current_time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: "chapter-1".to_string(),
        comic_id: "comic-1".to_string(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "chapter".to_string(),
        page_count: 1,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: StageMask::try_from(0_u32).unwrap(),
        creator_id: "translator-1".to_string(),
        creator: None,
        created_at: current_time,
        updated_at: current_time,
    }
}

// Build baseline page fixture with zero-unit counters.
fn page() -> PageInfo {
    //
    // Build a baseline page fixture for unit-chain assembly.
    let current_time = OffsetDateTime::now_utc();

    PageInfo {
        id: "page-1".to_string(),
        chapter_id: "chapter-1".to_string(),
        index: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: current_time,
        updated_at: current_time,
    }
}

// Build baseline assignment fixture for user-role scenarios.
fn assignment(roles: RoleMask) -> AssignmentInfo {
    //
    // Build an assignment fixture tied to the test translator.
    let current_time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: "assignment-1".to_string(),
        chapter_id: "chapter-1".to_string(),
        user_id: "translator-1".to_string(),
        user: None,
        chapter: None,
        roles,
        created_at: current_time,
        updated_at: current_time,
    }
}
