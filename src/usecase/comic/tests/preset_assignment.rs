use super::*;

#[tokio::test]
async fn create_rejects_preset_role_missing_from_membership() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    let mut instr = create_instr("workset-1");

    instr.preset_assignment_roles = Some(RoleMask::from(RoleField::TRANSLATOR));

    let err = create((&mock, &mock), token("user-1"), instr)
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert!(snapshot.comics.is_empty());

    assert!(snapshot.chapters.is_empty());

    assert!(snapshot.assignments.is_empty());
}

// creation(create)(positive): no preset leaves the first chapter unassigned while recording creation.
#[tokio::test]
async fn create_without_preset_preserves_creation_history() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    let created =
        create((&mock, &mock), token("user-1"), create_instr("workset-1"))
            .await
            .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.assignments.is_empty());

    assert!(snapshot.chapter_workflow_records.iter().any(|record| {
        record.chapter_id == created.chapter_id && matches!(
            record.payload,
            crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload::ChapterCreated
        )
    }));
}

// creation(create)(positive): first chapters retain explicit titles and exact actor-attributed history.
#[tokio::test]
async fn create_preserves_first_chapter_title_and_history() {
    for subtitle in [None, Some("Opening chapter".to_owned())] {
        let mock = Mock::new();

        mock.seed_workset(workset("workset-1", "team-1"));

        mock.seed_member(admin_member("user-1", "team-1"));

        let mut instr = create_instr("workset-1");

        instr.first_chapter_subtitle = subtitle.clone();

        let created = create((&mock, &mock), token("user-1"), instr)
            .await
            .unwrap();

        let snapshot = mock.snapshot();

        assert_eq!(snapshot.chapters.len(), 1);

        let chapter_info = &snapshot.chapters[0];

        assert_eq!(chapter_info.id, created.chapter_id);

        assert_eq!(chapter_info.comic_id, created.id);

        assert_eq!(chapter_info.creator_id, "user-1");

        assert_eq!(chapter_info.index, 0);

        assert!(chapter_info.is_pinned);

        match subtitle {
            Some(subtitle) => assert_eq!(chapter_info.subtitle, subtitle),
            None => assert_eq!(chapter_info.subtitle, "第1话"),
        }

        assert_eq!(snapshot.comics[0].chapter_count, 1);

        assert!(snapshot.assignments.is_empty());

        assert_eq!(snapshot.chapter_workflow_records.len(), 1);

        let record = &snapshot.chapter_workflow_records[0];

        assert_eq!(record.chapter_id, created.chapter_id);

        assert_eq!(record.actor_user_id.as_deref(), Some("user-1"));

        assert!(matches!(record.payload, crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload::ChapterCreated));
    }
}

// creation(create)(negative): non-admin creators and management presets are rejected before writes.
#[tokio::test]
async fn create_rejects_non_admin_and_management_presets() {
    for (membership, preset, expected) in [
        (RoleField::TRANSLATOR, None, ExpectedVariant::Perm),
        (
            RoleField::ADMIN,
            Some(RoleMask::from(RoleField::ADMIN)),
            ExpectedVariant::Args,
        ),
    ] {
        let mock = Mock::new();

        mock.seed_workset(workset("workset-1", "team-1"));

        let mut member_info = admin_member("user-1", "team-1");

        member_info.roles = RoleMask::from(membership);

        mock.seed_member(member_info);

        let mut instr = create_instr("workset-1");

        instr.preset_assignment_roles = preset;

        let err = create((&mock, &mock), token("user-1"), instr)
            .await
            .err()
            .unwrap();

        assert_expected_variant(err, expected);

        let snapshot = mock.snapshot();

        assert!(snapshot.comics.is_empty());

        assert!(snapshot.chapters.is_empty());

        assert!(snapshot.assignments.is_empty());

        assert!(snapshot.chapter_workflow_records.is_empty());

        assert_eq!(snapshot.worksets[0].comic_count, 0);
    }
}
