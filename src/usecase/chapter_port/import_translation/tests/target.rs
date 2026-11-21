// import_translation(target)(positive): the selected target writes only its text fields.
// import_translation(target)(negative): a target without its matching role or a LabelPlus both-target import is rejected.

use super::*;

// Single-populated-page LabelPlus fixture for import target tests.
const LABEL_PLUS_SINGLE_PAGE: &str = concat!(
    "1,0\n",
    "-\n",
    "框内\n",
    "框外\n",
    "-\n",
    "note\n",
    ">>>>>>>>[000.jpg]<<<<<<<<\n",
    "----------------[1]----------------[0.1,0.2,1]\n",
    "imported text\n",
);

// Single-page PopRaKo document carrying both text fields.
const POPRAKO_SINGLE_PAGE: &str = r#"{
    "chapter_id": "chapter--00000000001",
    "chapter_index": 0,
    "chapter_subtitle": null,
    "comic_id": "comic--00000000001",
    "comic_title": "title",
    "pages": [
        {
            "page_id": "page_source--00000000001",
            "page_index": 0,
            "units": [
                {
                    "unit_id": "unit--00000000001",
                    "unit_index": 0,
                    "page_id": "page_source--00000000001",
                    "page_index": 0,
                    "x_coord": 0.1,
                    "y_coord": 0.2,
                    "is_bubble": true,
                    "translated_text": "translated",
                    "translator_id": null,
                    "is_proofread": true,
                    "proofread_text": "proofread",
                    "proofreader_id": null
                }
            ]
        }
    ]
}"#;

// Replace the baseline assignment with the roles required by one target.
fn seed_import_roles(mock: &Mock, roles: RoleMask) {
    mock.state.lock().unwrap().assignments.clear();

    mock.seed_assignment(assignment(
        "chapter--00000000001",
        "user--00000000001",
        roles,
    ));
}

#[tokio::test]
async fn import_label_plus_target_translation_writes_translated_text_only() {
    //
    // Verify a translator-targeted LabelPlus import leaves proofread fields empty.
    let mock = Mock::new();

    seed_base(&mock, 1, 0, 0);

    seed_import_roles(&mock, RoleMask::from(RoleField::TRANSLATOR));

    mock.seed_page(page("page_source--00000000001", 0, 0, 0));

    import_translation(
        (&mock, &mock),
        token("user--00000000001"),
        ImportChapterTranslationInstr {
            format: TranslationFormat::LabelPlus.into(),
            mode: ChapterTranslationImportMode::Overwrite.into(),
            target: ChapterTranslationImportTarget::Translation.into(),
            content: LABEL_PLUS_SINGLE_PAGE.into(),
        },
        "chapter--00000000001".into(),
    )
    .await
    .expect("translator should import translation text");

    let snapshot = mock.snapshot();

    let unit = snapshot.units.first().unwrap();

    assert_eq!(unit.translated_text.as_deref(), Some("imported text"));

    assert!(unit.proofread_text.is_none());

    assert!(!unit.is_proofread);

    assert_eq!(
        unit.last_translator_id.as_deref(),
        Some("user--00000000001")
    );

    assert!(unit.last_proofreader_id.is_none());
}

#[tokio::test]
async fn import_poprako_writes_only_selected_text_field() {
    //
    // Verify PopRaKo imports ignore the text field that the target excludes.
    for (target, expect_translation) in [
        (ChapterTranslationImportTarget::Translation, true),
        (ChapterTranslationImportTarget::Proofread, false),
    ] {
        let mock = Mock::new();

        seed_base(&mock, 1, 0, 0);

        let role = if expect_translation {
            RoleField::TRANSLATOR
        } else {
            RoleField::PROOFREADER
        };

        seed_import_roles(&mock, RoleMask::from(role));

        mock.seed_page(page("page_source--00000000001", 0, 0, 0));

        import_translation(
            (&mock, &mock),
            token("user--00000000001"),
            ImportChapterTranslationInstr {
                format: TranslationFormat::PopRaKo.into(),
                mode: ChapterTranslationImportMode::Overwrite.into(),
                target: target.into(),
                content: POPRAKO_SINGLE_PAGE.into(),
            },
            "chapter--00000000001".into(),
        )
        .await
        .expect("target should match the seeded role");

        let snapshot = mock.snapshot();

        let unit = snapshot.units.first().unwrap();

        assert_eq!(unit.translated_text.is_some(), expect_translation);

        assert_eq!(unit.proofread_text.is_some(), !expect_translation);

        assert_eq!(unit.is_proofread, !expect_translation);
    }
}

#[tokio::test]
async fn import_poprako_target_both_writes_both_text_fields() {
    //
    // Verify both-targeted PopRaKo imports keep translation and proofread text.
    let mock = Mock::new();

    seed_base(&mock, 1, 0, 0);

    seed_import_roles(
        &mock,
        RoleMask::from(RoleField::TRANSLATOR)
            .union(RoleMask::from(RoleField::PROOFREADER)),
    );

    mock.seed_page(page("page_source--00000000001", 0, 0, 0));

    import_translation(
        (&mock, &mock),
        token("user--00000000001"),
        ImportChapterTranslationInstr {
            format: TranslationFormat::PopRaKo.into(),
            mode: ChapterTranslationImportMode::Overwrite.into(),
            target: ChapterTranslationImportTarget::Both.into(),
            content: POPRAKO_SINGLE_PAGE.into(),
        },
        "chapter--00000000001".into(),
    )
    .await
    .expect("both roles should import both text fields");

    let snapshot = mock.snapshot();

    let unit = snapshot.units.first().unwrap();

    assert_eq!(unit.translated_text.as_deref(), Some("translated"));

    assert_eq!(unit.proofread_text.as_deref(), Some("proofread"));

    assert!(unit.is_proofread);
}

#[tokio::test]
async fn import_rejects_target_without_matching_role() {
    //
    // Verify the selected target must be covered by the current assignment.
    for (role, target) in [
        (
            RoleField::PROOFREADER,
            ChapterTranslationImportTarget::Translation,
        ),
        (
            RoleField::TRANSLATOR,
            ChapterTranslationImportTarget::Proofread,
        ),
        (RoleField::PROOFREADER, ChapterTranslationImportTarget::Both),
    ] {
        let mock = Mock::new();

        seed_base(&mock, 1, 0, 0);

        seed_import_roles(&mock, RoleMask::from(role));

        mock.seed_page(page("page_source--00000000001", 0, 0, 0));

        let error = import_translation(
            (&mock, &mock),
            token("user--00000000001"),
            ImportChapterTranslationInstr {
                format: TranslationFormat::LabelPlus.into(),
                mode: ChapterTranslationImportMode::Overwrite.into(),
                target: target.into(),
                content: LABEL_PLUS_SINGLE_PAGE.into(),
            },
            "chapter--00000000001".into(),
        )
        .await
        .err()
        .unwrap();

        assert_expected_variant(&error, ExpectedVariant::Perm);

        assert!(mock.snapshot().units.is_empty());
    }
}

#[tokio::test]
async fn import_rejects_label_plus_target_both() {
    //
    // Verify LabelPlus cannot write both text fields even with both roles.
    let mock = Mock::new();

    seed_base(&mock, 1, 0, 0);

    seed_import_roles(
        &mock,
        RoleMask::from(RoleField::TRANSLATOR)
            .union(RoleMask::from(RoleField::PROOFREADER)),
    );

    mock.seed_page(page("page_source--00000000001", 0, 0, 0));

    let error = import_translation(
        (&mock, &mock),
        token("user--00000000001"),
        ImportChapterTranslationInstr {
            format: TranslationFormat::LabelPlus.into(),
            mode: ChapterTranslationImportMode::Overwrite.into(),
            target: ChapterTranslationImportTarget::Both.into(),
            content: LABEL_PLUS_SINGLE_PAGE.into(),
        },
        "chapter--00000000001".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);

    assert!(mock.snapshot().units.is_empty());
}
