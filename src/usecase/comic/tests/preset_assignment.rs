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
