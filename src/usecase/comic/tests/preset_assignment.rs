use super::*;

#[tokio::test]
async fn create_rejects_preset_role_missing_from_membership() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));

    let mut instr = create_instr("workset--00000000001");

    instr.preset_assignment_roles = Some(RoleMask::from(RoleField::TRANSLATOR));

    let error = create((&mock, &mock), token("user--00000000001"), instr)
        .await
        .err()
        .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert!(snapshot.comics.is_empty());

    assert!(snapshot.chapters.is_empty());

    assert!(snapshot.assignments.is_empty());
}

// creation(create)(positive): no preset leaves the first chapter unassigned while recording creation.
#[tokio::test]
async fn create_without_preset_preserves_creation_history() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));

    let created = create(
        (&mock, &mock),
        token("user--00000000001"),
        create_instr("workset--00000000001"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.assignments.is_empty());

    assert!(snapshot.chapter_workflow_records.iter().any(|record| {
        record.chapter_id == created.chapter_id
            && matches!(
            record.payload,
            crate::value::chapter_workflow_record::
                ChapterWorkflowRecordPayload::ChapterCreated
        )
    }));
}

// creation(create)(positive): first chapters retain explicit titles and exact actor-attributed history.
#[tokio::test]
async fn create_preserves_first_chapter_title_and_history() {
    for subtitle in [None, Some("Opening chapter".to_owned())] {
        let mock = Mock::new();

        mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

        mock.seed_member(admin_member(
            "user--00000000001",
            "team--00000000001",
        ));

        let mut instr = create_instr("workset--00000000001");

        instr.first_chapter_subtitle = subtitle.clone();

        let created = create((&mock, &mock), token("user--00000000001"), instr)
            .await
            .unwrap();

        let snapshot = mock.snapshot();

        assert_eq!(snapshot.chapters.len(), 1);

        let chapter_info = snapshot.chapters.first().unwrap();

        assert_eq!(chapter_info.id, created.chapter_id);

        assert_eq!(chapter_info.comic_id, created.id);

        assert_eq!(chapter_info.creator_id, "user--00000000001");

        assert_eq!(chapter_info.index, 0);

        assert!(chapter_info.is_pinned);

        match subtitle {
            Some(subtitle) => assert_eq!(chapter_info.subtitle, subtitle),
            None => assert_eq!(chapter_info.subtitle, "第1话"),
        }

        assert_eq!(snapshot.comics.first().unwrap().chapter_count, 1);

        assert!(snapshot.assignments.is_empty());

        assert_eq!(snapshot.chapter_workflow_records.len(), 1);

        let record = snapshot.chapter_workflow_records.first().unwrap();

        assert_eq!(record.chapter_id, created.chapter_id);

        assert_eq!(record.actor_user_id.as_deref(), Some("user--00000000001"));

        assert!(matches!(
            record.payload,
            crate::value::chapter_workflow_record::
                ChapterWorkflowRecordPayload::ChapterCreated
        ));
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

        mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

        let mut member_info =
            admin_member("user--00000000001", "team--00000000001");

        member_info.roles = RoleMask::from(membership);

        mock.seed_member(member_info);

        let mut instr = create_instr("workset--00000000001");

        instr.preset_assignment_roles = preset;

        let error = create((&mock, &mock), token("user--00000000001"), instr)
            .await
            .err()
            .unwrap();

        assert_expected_variant(&error, expected);

        let snapshot = mock.snapshot();

        assert!(snapshot.comics.is_empty());

        assert!(snapshot.chapters.is_empty());

        assert!(snapshot.assignments.is_empty());

        assert!(snapshot.chapter_workflow_records.is_empty());

        assert_eq!(snapshot.worksets.first().unwrap().comic_count, 0);
    }
}
