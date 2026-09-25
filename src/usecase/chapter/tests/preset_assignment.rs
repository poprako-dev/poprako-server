use super::*;

use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;

#[tokio::test]
async fn create_rejects_preset_role_missing_from_membership() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));

    let err = create(
        (&mock, &mock),
        token("user-1"),
        CreateChapterInstr {
            comic_id: "comic-1".into(),
            subtitle: None,
            preset_assignment_roles: Some(RoleMask::from(
                RoleField::TRANSLATOR,
            )),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert!(mock.snapshot().chapters.is_empty());

    assert!(mock.snapshot().assignments.is_empty());
}

// creation(create)(positive): creation preserves naming, counters, pins, assignments, and exact history.
#[tokio::test]
async fn create_completes_chapter_obligations() {
    for previous_pin in [false, true] {
        for subtitle in [None, Some("Next chapter".to_owned())] {
            let mock = Mock::new();

            seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));

            mock.seed_chapter(chapter("chapter-1", "comic-1", 0, previous_pin));

            mock.seed_chapter(chapter("chapter-2", "comic-1", 1, false));

            mock.state.lock().unwrap().comics[0].last_active_at =
                time::OffsetDateTime::UNIX_EPOCH;

            let created = create(
                (&mock, &mock),
                token("user-1"),
                CreateChapterInstr {
                    comic_id: "comic-1".into(),
                    subtitle: subtitle.clone(),
                    preset_assignment_roles: None,
                },
            )
            .await
            .unwrap();

            let snapshot = mock.snapshot();

            assert_eq!(snapshot.chapters.len(), 3);

            let chapter_info = snapshot
                .chapters
                .iter()
                .find(|chapter_info| chapter_info.id == created.id)
                .unwrap();

            assert_eq!(chapter_info.comic_id, "comic-1");

            assert_eq!(chapter_info.creator_id, "user-1");

            assert_eq!(chapter_info.index, 2);

            assert!(chapter_info.is_pinned);

            match subtitle {
                Some(subtitle) => assert_eq!(chapter_info.subtitle, subtitle),
                None => assert_eq!(chapter_info.subtitle, "第3话"),
            }

            assert_eq!(
                snapshot
                    .chapters
                    .iter()
                    .filter(|chapter_info| chapter_info.is_pinned)
                    .count(),
                1
            );

            assert_eq!(snapshot.comics[0].chapter_count, 3);

            assert!(
                snapshot.comics[0].last_active_at
                    > time::OffsetDateTime::UNIX_EPOCH
            );

            assert!(snapshot.assignments.is_empty());

            assert_eq!(
                snapshot.chapter_workflow_records.len(),
                1 + usize::from(previous_pin)
            );

            for record in &snapshot.chapter_workflow_records {
                assert_eq!(record.actor_user_id.as_deref(), Some("user-1"));

                match record.chapter_id == created.id {
                    true => assert!(matches!(
                        record.payload,
                        ChapterWorkflowRecordPayload::ChapterCreated
                    )),
                    false => {
                        assert_eq!(record.chapter_id, "chapter-1");

                        assert!(matches!(
                            record.payload,
                            ChapterWorkflowRecordPayload::ChapterUnpinned
                        ));
                    }
                }
            }
        }
    }
}

// creation(create)(negative): worker membership alone cannot create a Chapter.
#[tokio::test]
async fn create_rejects_non_admin() {
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::TRANSLATOR));

    let err = create(
        (&mock, &mock),
        token("user-1"),
        CreateChapterInstr {
            comic_id: "comic-1".into(),
            subtitle: None,
            preset_assignment_roles: None,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert!(snapshot.chapters.is_empty());

    assert!(snapshot.assignments.is_empty());

    assert!(snapshot.chapter_workflow_records.is_empty());

    assert_eq!(snapshot.comics[0].chapter_count, 2);
}
