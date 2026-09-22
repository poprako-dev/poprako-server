//! Team administration without chapter assignments.

use super::*;

use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;

// creation(create)(positive): only explicit worker presets create assignments.
#[tokio::test]
async fn create_only_assigns_explicit_worker_presets() {
    for preset in [None, Some(RoleMask::from(RoleField::TRANSLATOR))] {
        let mock = Mock::new();

        seed_scope(
            &mock,
            "user-1",
            RoleMask::from(RoleField::ADMIN)
                .union(RoleMask::from(RoleField::TRANSLATOR)),
        );

        let created = create(
            (&mock, &mock),
            token("user-1"),
            CreateChapterInstr {
                comic_id: "comic-1".into(),
                subtitle: None,
                preset_assignment_roles: preset,
            },
        )
        .await
        .unwrap();

        let snapshot = mock.snapshot();

        assert_eq!(snapshot.assignments.len(), usize::from(preset.is_some()));

        if let Some(roles) = preset {
            assert_eq!(snapshot.assignments[0].roles, roles);
        }

        assert!(snapshot.chapter_workflow_records.iter().any(|record| {
            record.chapter_id == created.id
                && matches!(
                    record.payload,
                    ChapterWorkflowRecordPayload::ChapterCreated
                )
        }));
    }
}

// administration(update_info)(positive): team admin manages a chapter without an assignment.
#[tokio::test]
async fn team_admin_manages_without_assignment() {
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    update_info(
        (&mock, &mock),
        token("user-1"),
        UpdateChapterInfoInstr {
            id: "chapter-1".into(),
            subtitle: Some("updated".into()),
        },
    )
    .await
    .unwrap();

    mark_pinned((&mock, &mock), token("user-1"), "chapter-1".into())
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.assignments.is_empty());

    assert_eq!(snapshot.chapters[0].subtitle, "updated");

    assert!(snapshot.chapters[0].is_pinned);

    mock.state.lock().unwrap().members.clear();

    mock.seed_member(member(
        "user-1",
        "team-1",
        RoleMask::from(RoleField::TRANSLATOR),
    ));

    let err = update_info(
        (&mock, &mock),
        token("user-1"),
        UpdateChapterInfoInstr {
            id: "chapter-1".into(),
            subtitle: Some("forbidden".into()),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert_eq!(mock.snapshot().chapters[0].subtitle, "updated");
}

// administration(mark_pinned)(negative): another team's admin cannot manage this chapter.
#[tokio::test]
async fn cross_team_admin_cannot_manage_chapter() {
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));

    mock.seed_member(member(
        "user-2",
        "team-2",
        RoleMask::from(RoleField::ADMIN),
    ));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    let err = mark_pinned((&mock, &mock), token("user-2"), "chapter-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert!(!mock.snapshot().chapters[0].is_pinned);
}

// creation(create)(negative): chapter presets cannot contain a team management role.
#[tokio::test]
async fn create_rejects_admin_preset() {
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));

    let err = create(
        (&mock, &mock),
        token("user-1"),
        CreateChapterInstr {
            comic_id: "comic-1".into(),
            subtitle: None,
            preset_assignment_roles: Some(RoleMask::from(RoleField::ADMIN)),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(mock.snapshot().chapters.is_empty());
}
