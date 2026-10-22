// import_translation(notes)(positive): an annotation-only import counts as translated without fabricated proofreading.

use super::*;

use crate::value::chapter::stage::{Stage, StagePhase};

#[tokio::test]
async fn annotation_only_import_starts_translation_for_either_editor() {
    for role in [RoleField::TRANSLATOR, RoleField::PROOFREADER] {
        let mock = Mock::new();

        seed_base(&mock, 1, 0, 0);

        mock.state.lock().unwrap().assignments.clear();

        mock.seed_assignment(assignment(
            "chapter-1",
            "user-1",
            RoleMask::from(role),
        ));

        mock.seed_page(page("page-1", 0, 0, 0));

        let content = "1,0\n-\n框内\n框外\n-\nnotes\n>>>>>>>>[000.jpg]<<<<<<<<\n----------------[1]----------------[0.1,0.2,1]\n###小字###\n";

        import_translation(
            (&mock, &mock),
            token("user-1"),
            ImportChapterTranslationInstr {
                format: TranslationFormat::LabelPlus.into(),
                mode: ChapterTranslationImportMode::Overwrite.into(),
                content: content.into(),
            },
            "chapter-1".into(),
        )
        .await
        .unwrap();

        let snapshot = mock.snapshot();

        let unit = snapshot.units.first().unwrap();

        assert_eq!(unit.typography_note.as_deref(), Some("小字"));

        assert!(unit.translation_note.is_none());

        assert!(unit.translated_text.is_none());

        assert!(unit.proofread_text.is_none());

        assert!(!unit.is_proofread);

        assert!(unit.last_translator_id.is_none());

        assert!(unit.last_proofreader_id.is_none());

        assert_eq!(snapshot.pages.first().unwrap().translated_unit_count, 1);

        assert_eq!(snapshot.chapters.first().unwrap().translated_unit_count, 1);

        assert_eq!(snapshot.pages.first().unwrap().proofread_unit_count, 0);

        assert_eq!(
            snapshot
                .chapters
                .first()
                .unwrap()
                .stages
                .get_phase(Stage::Translate),
            StagePhase::Active
        );
    }
}
