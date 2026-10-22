use super::*;

use crate::part_impl::repo::mock_impl::MockSnapshot;

// Builds a selected transform for one persisted Unit.
fn transform_instr(
    unit_id: &str,
    origin: &str,
    target: &str,
) -> UnitTransformInstr {
    UnitTransformInstr {
        unit_id: unit_id.to_string(),
        transforms: vec![UnitTextTransformInstr {
            origin: origin.to_string(),
            target: target.to_string(),
        }],
    }
}

// Checks Page counters against visible Units and Chapter counters against Pages.
fn assert_count_agreement(snapshot: &MockSnapshot) {
    for page_info in &snapshot.pages {
        let visible_units = snapshot
            .units
            .iter()
            .filter(|unit_info| {
                unit_info.page_id == page_info.id
                    && unit_info.hidden_at.is_none()
            })
            .collect::<Vec<_>>();

        assert_eq!(page_info.total_unit_count, visible_units.len());

        assert_eq!(
            page_info.translated_unit_count,
            visible_units
                .iter()
                .filter(|unit_info| unit_info.is_translated())
                .count()
        );

        assert_eq!(
            page_info.proofread_unit_count,
            visible_units
                .iter()
                .filter(|unit_info| unit_info.is_proofread)
                .count()
        );
    }

    let chapter_info = snapshot.chapters.first().unwrap();

    assert_eq!(
        chapter_info.total_unit_count,
        snapshot
            .pages
            .iter()
            .map(|page_info| page_info.total_unit_count)
            .sum::<usize>()
    );

    assert_eq!(
        chapter_info.translated_unit_count,
        snapshot
            .pages
            .iter()
            .map(|page_info| page_info.translated_unit_count)
            .sum::<usize>()
    );

    assert_eq!(
        chapter_info.proofread_unit_count,
        snapshot
            .pages
            .iter()
            .map(|page_info| page_info.proofread_unit_count)
            .sum::<usize>()
    );
}

// Builds two Pages with one translated, approved Unit on each.
async fn two_page_scope() -> Mock {
    let roles = RoleMask::from(RoleField::TRANSLATOR)
        .union(RoleMask::from(RoleField::PROOFREADER));

    let mock = save_scope(roles);

    let mut second_page = page();

    second_page.id = "page-2".to_string();

    second_page.index = 1;

    mock.seed_page(second_page);

    mock.state
        .lock()
        .unwrap()
        .chapters
        .first_mut()
        .unwrap()
        .page_count = 2;

    for (page_id, text) in [("page-1", "alpha"), ("page-2", "abcd")] {
        let mut instr =
            save_instr(vec![create_with_revision("local", Some(text))]);

        instr.page_id = page_id.to_string();

        save_edits((&mock, &mock), token("translator-1"), instr)
            .await
            .unwrap();
    }

    mock
}

// save_edits/transform(page_edit)(positive): counters agree through text, visibility, and approval changes.
#[tokio::test]
async fn page_and_chapter_counts_agree_after_save_and_transform() {
    let mock = two_page_scope().await;

    assert_count_agreement(&mock.snapshot());

    for part in [UnitTextPart::TranslatedText, UnitTextPart::ProofreadText] {
        let snapshot = mock.snapshot();

        let units = snapshot
            .units
            .iter()
            .map(|unit_info| {
                let origin = match part {
                    UnitTextPart::TranslatedText => {
                        unit_info.translated_text.as_deref().unwrap()
                    }
                    UnitTextPart::ProofreadText => "proofread",
                };

                transform_instr(&unit_info.id, origin, "")
            })
            .collect();

        transform(
            (&mock, &mock),
            token("translator-1"),
            "chapter-1".to_string(),
            TransformChapterUnitsInstr { part, units },
        )
        .await
        .unwrap();

        assert_count_agreement(&mock.snapshot());
    }

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.chapters.first().unwrap().translated_unit_count, 0);

    assert_eq!(snapshot.chapters.first().unwrap().proofread_unit_count, 2);

    let unit_id = snapshot.units.first().unwrap().id.clone();

    for edit in [
        UnitEditInstr::Delete {
            id: unit_id.clone(),
        },
        UnitEditInstr::Patch {
            id: unit_id,
            next_id: Patch::Skip,
            is_bubble: None,
            is_flagged: None,
            coord: None,
            typography_note: Patch::Skip,
            translation_note: Patch::Skip,
            translation: Patch::Skip,
            revision: Patch::Skip,
        },
    ] {
        save_edits(
            (&mock, &mock),
            token("translator-1"),
            save_instr(vec![edit]),
        )
        .await
        .unwrap();

        assert_count_agreement(&mock.snapshot());
    }
}

// transform(page_edit)(negative): a later Page failure rolls back previously persisted Page changes.
#[tokio::test]
async fn later_page_transform_failure_rolls_back_prior_page_edits_and_counters()
{
    let mock = two_page_scope().await;

    let unit_id = mock.snapshot().units.first().unwrap().id.clone();

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![UnitEditInstr::Patch {
            id: unit_id,
            next_id: Patch::Skip,
            is_bubble: None,
            is_flagged: None,
            coord: None,
            typography_note: Patch::Skip,
            translation_note: Patch::Skip,
            translation: Patch::Skip,
            revision: Patch::Clear,
        }]),
    )
    .await
    .unwrap();

    let before = mock.snapshot();

    let units = before
        .units
        .iter()
        .map(|unit_info| {
            let transforms = match unit_info.page_id.as_str() {
                "page-1" => vec![UnitTextTransformInstr {
                    origin: "alpha".to_string(),
                    target: String::new(),
                }],
                _ => vec![
                    UnitTextTransformInstr {
                        origin: "abc".to_string(),
                        target: "first".to_string(),
                    },
                    UnitTextTransformInstr {
                        origin: "bcd".to_string(),
                        target: "second".to_string(),
                    },
                ],
            };

            UnitTransformInstr {
                unit_id: unit_info.id.clone(),
                transforms,
            }
        })
        .collect();

    let error = transform(
        (&mock, &mock),
        token("translator-1"),
        "chapter-1".to_string(),
        TransformChapterUnitsInstr {
            part: UnitTextPart::TranslatedText,
            units,
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

    let after = mock.snapshot();

    for (before, after) in before.units.iter().zip(&after.units) {
        assert_eq!(before.translated_text, after.translated_text);

        assert_eq!(before.updated_at, after.updated_at);
    }

    for (before, after) in before.pages.iter().zip(&after.pages) {
        assert_eq!(before.updated_at, after.updated_at);
    }

    assert_eq!(
        before.chapters.first().unwrap().updated_at,
        after.chapters.first().unwrap().updated_at
    );

    assert_eq!(
        before.comics.first().unwrap().last_active_at,
        after.comics.first().unwrap().last_active_at
    );

    assert_eq!(
        before.chapter_workflow_records.len(),
        after.chapter_workflow_records.len()
    );

    assert_count_agreement(&after);
}

// save_edits/transform(page_edit)(positive): only sparse save completes activity and token effects without Unit writes.
#[tokio::test]
async fn no_op_transform_and_sparse_save_keep_distinct_completion_effects() {
    let mock = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    save_edits(
        (&mock, &mock),
        token("translator-1"),
        save_instr(vec![create("local", None, Some("text"))]),
    )
    .await
    .unwrap();

    {
        let mut state = mock.state.lock().unwrap();

        state.pages.first_mut().unwrap().updated_at =
            OffsetDateTime::UNIX_EPOCH;

        state.comics.first_mut().unwrap().last_active_at =
            OffsetDateTime::UNIX_EPOCH;
    }

    let before = mock.snapshot();

    let unit_id = before.units.first().unwrap().id.clone();

    transform(
        (&mock, &mock),
        token("translator-1"),
        "chapter-1".to_string(),
        TransformChapterUnitsInstr {
            part: UnitTextPart::TranslatedText,
            units: vec![transform_instr(&unit_id, "missing", "replacement")],
        },
    )
    .await
    .unwrap();

    let after_transform = mock.snapshot();

    assert_eq!(
        after_transform.pages.first().unwrap().updated_at,
        OffsetDateTime::UNIX_EPOCH
    );

    assert_eq!(
        after_transform.comics.first().unwrap().last_active_at,
        OffsetDateTime::UNIX_EPOCH
    );

    assert_eq!(
        after_transform.chapters.first().unwrap().updated_at,
        before.chapters.first().unwrap().updated_at
    );

    assert_eq!(mock.unit_saves.token_count(), 1);

    let instr = save_instr(vec![UnitEditInstr::Patch {
        id: unit_id,
        next_id: Patch::Skip,
        is_bubble: None,
        is_flagged: None,
        coord: None,
        typography_note: Patch::Skip,
        translation_note: Patch::Skip,
        translation: Patch::Skip,
        revision: Patch::Skip,
    }]);

    let save_id = instr.save_id.clone();

    let saved = save_edits((&mock, &mock), token("translator-1"), instr)
        .await
        .unwrap();

    assert!(saved.created_unit_ids.is_empty());

    let after_save = mock.snapshot();

    assert!(
        after_save.pages.first().unwrap().updated_at
            > OffsetDateTime::UNIX_EPOCH
    );

    assert!(
        after_save.comics.first().unwrap().last_active_at
            > OffsetDateTime::UNIX_EPOCH
    );

    assert_eq!(
        after_save.units.first().unwrap().updated_at,
        before.units.first().unwrap().updated_at
    );

    assert_eq!(mock.unit_saves.token_count(), 2);

    let mut replay =
        save_instr(vec![create("replayed", None, Some("ignored"))]);

    replay.save_id = save_id;

    assert_eq!(
        save_edits((&mock, &mock), token("translator-1"), replay)
            .await
            .unwrap(),
        saved
    );

    assert_eq!(
        mock.snapshot().pages.first().unwrap().updated_at,
        after_save.pages.first().unwrap().updated_at
    );

    assert_eq!(mock.snapshot().units.len(), 1);

    assert_count_agreement(&mock.snapshot());
}
