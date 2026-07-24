use super::*;

#[tokio::test]
async fn update_stage_admin_advances_any_stage() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-2",
        RoleMask::from(RoleField::PUBLISHER),
    ));

    update_stage(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterStageParams {
            id: "chapter-1".into(),
            stage: Stage::Publish,
            oper: StageOper::Advance,
        },
    )
    .await
    .ok()
    .unwrap();

    let snapshot = mock.snapshot();

    let stages = &snapshot.chapters[0].stages;

    assert_eq!(stages.get_phase(Stage::Publish), StagePhase::Completed);
}

#[tokio::test]
async fn update_stage_rejects_reviewer_outside_review_stage() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::REVIEWER));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::REVIEWER),
    ));

    let err = update_stage(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterStageParams {
            id: "chapter-1".into(),
            stage: Stage::Translate,
            oper: StageOper::Advance,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn update_stage_rejects_invalid_transition() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::PUBLISHER));

    let mut chapter_info = chapter("chapter-1", "comic-1", 1, false);

    chapter_info.stages = chapter_info
        .stages
        .try_set_phase(Stage::Publish, StagePhase::Completed)
        .ok()
        .unwrap();

    mock.seed_chapter(chapter_info);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::PUBLISHER),
    ));

    let err = update_stage(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterStageParams {
            id: "chapter-1".into(),
            stage: Stage::Publish,
            oper: StageOper::Advance,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn update_stage_publish_enqueues_page_image_delete() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::PUBLISHER));

    mock.seed_chapter(chapter("chapter-1", "comic-1", 1, false));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::PUBLISHER),
    ));

    mock.seed_page(page("page-1", "chapter-1", Some("page-1.png")));

    update_stage(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterStageParams {
            id: "chapter-1".into(),
            stage: Stage::Publish,
            oper: StageOper::Advance,
        },
    )
    .await
    .ok()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.prom_records.len(), 1);

    let Payload::Image(ImagePayload::Delete { object_key }) =
        snapshot.prom_records[0].payload()
    else {
        panic!("expected image delete payload");
    };

    assert_eq!(object_key, "page-1.png");

    assert_eq!(snapshot.pages[0].image_key, None);

    assert!(!snapshot.pages[0].image_uploaded);

    assert_eq!(snapshot.pages[0].image_version, 2);

    assert_eq!(snapshot.pages[0].image_hash, ImageHash::new([0; 32]));

    assert_eq!(snapshot.pages[0].image_ext, ImageExt::Png);

    let events = mock.drain_events();

    assert_eq!(events.len(), 2);

    assert!(matches!(events[0], Event::ChapterWorkflowCompleted(_)));

    assert!(matches!(events[1], Event::ChapterPublished(_)));
}

#[tokio::test]
async fn published_chapter_rejects_metadata_and_stage_updates() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));

    let mut chapter_info = chapter("chapter-1", "comic-1", 0, false);

    chapter_info.stages = chapter_info
        .stages
        .try_set_phase(Stage::Publish, StagePhase::Completed)
        .unwrap();

    mock.seed_chapter(chapter_info);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    let info_result = update_info(
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterInfoParams {
            id: "chapter-1".into(),
            subtitle: Some("changed".into()),
            pin: None,
        },
    )
    .await;

    assert!(matches!(info_result, Err(BaseError::Expected { .. })));

    let stage_result = update_stage(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        UpdateChapterStageParams {
            id: "chapter-1".into(),
            stage: Stage::Translate,
            oper: StageOper::Advance,
        },
    )
    .await;

    assert!(matches!(stage_result, Err(BaseError::Expected { .. })));

    assert_eq!(mock.snapshot().chapters[0].subtitle, "chapter 0");
}
