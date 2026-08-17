use super::*;

#[tokio::test]
async fn reserve_chapter_pages_creates_pages_and_urls() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let before = OffsetDateTime::now_utc();

    let reserved = reserve_chapter_pages(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        ReserveChapterPagesInstr {
            chapter_id: "chapter-1".into(),
            pages: vec![
                PageImageInstr {
                    page_id: None,
                    image_hash: ImageHash::new([0u8; 32]),
                    new_byte_len: Some(4096),
                    ext: ImageExt::Png,
                },
                PageImageInstr {
                    page_id: None,
                    image_hash: ImageHash::new([0u8; 32]),
                    new_byte_len: Some(4096),
                    ext: ImageExt::Png,
                },
            ],
        },
    )
    .await;

    assert!(reserved.is_ok());

    let reserved = reserved.ok().unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(reserved.pages.len(), 2);

    assert_eq!(snapshot.pages.len(), 2);

    assert_eq!(snapshot.pages[0].index, 0);

    assert_eq!(snapshot.pages[1].index, 1);

    assert_eq!(snapshot.pages[0].image_version, Some(1));

    assert_eq!(snapshot.pages[1].image_version, Some(1));

    assert_eq!(reserved.pages[0].slot.as_ref().unwrap().image_version, 1);

    assert_eq!(reserved.pages[1].slot.as_ref().unwrap().image_version, 1);

    assert_eq!(snapshot.chapters[0].page_count, 2);

    assert_eq!(snapshot.prom_records.len(), 3);

    let advance_record = snapshot
        .prom_records
        .iter()
        .find(|record| {
            matches!(
                record.payload(),
                TaskPayload::Chapter(ChapterPayload::TryAdvanceRawProvideStage {
                    chapter_id,
                    ..
                }) if chapter_id == "chapter-1"
            )
        })
        .unwrap();

    assert!(advance_record.visible_at() - before >= TimeDuration::minutes(20));

    assert!(
        reserved.pages[0]
            .slot
            .as_ref()
            .unwrap()
            .put_url
            .contains("https://test.local/put/page/chapter_chapter-1/")
    );

    for creation in &reserved.pages {
        //
        let page_info = snapshot
            .pages
            .iter()
            .find(|page_info| page_info.id == creation.page_id)
            .unwrap();

        let object_key = page_info.image_key.as_deref().unwrap();

        assert!(object_key.ends_with("-1.png"));

        assert_one_image_check_record(
            &snapshot.prom_records,
            ResourceKind::PageImage,
            &creation.page_id,
            object_key,
            creation.slot.as_ref().unwrap().image_version,
        );
    }

    process_pending(&mock).await.ok().unwrap();

    let processed = mock.snapshot();

    assert!(
        processed
            .pages
            .iter()
            .all(|page_info| page_info.is_image_uploaded == Some(true))
    );

    assert!(
        processed.chapters[0]
            .stages
            .has_phase(Stage::RawProvide, StagePhase::Completed)
    );
}

#[tokio::test]
async fn reserve_chapter_pages_replaces_existing_manifest() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    mock.seed_page(page(
        "page-1",
        0,
        Some("page/chapter_chapter-1/page-1-1.png"),
        true,
        1,
    ));

    let result = reserve_chapter_pages(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        ReserveChapterPagesInstr {
            chapter_id: "chapter-1".into(),
            pages: vec![
                PageImageInstr {
                    page_id: Some("page-1".into()),
                    image_hash: ImageHash::new([0; 32]),
                    new_byte_len: None,
                    ext: ImageExt::Png,
                },
                PageImageInstr {
                    page_id: None,
                    image_hash: ImageHash::new([1; 32]),
                    new_byte_len: Some(4096),
                    ext: ImageExt::Png,
                },
            ],
        },
    )
    .await;

    assert!(result.is_ok());

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 2);

    assert_eq!(snapshot.pages[0].id, "page-1");

    assert_eq!(snapshot.pages[0].index, 0);

    assert_eq!(snapshot.pages[1].index, 1);

    let payload = result.unwrap();

    assert_eq!(payload.pages[0].page_id, "page-1");

    assert!(payload.pages[0].slot.is_none());
}

#[tokio::test]
async fn reserve_chapter_pages_preserves_pending_page_without_new_byte_len() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    mock.seed_page(page(
        "page-1",
        0,
        Some("page/chapter_chapter-1/page-1-2.png"),
        false,
        2,
    ));

    let reserved = reserve_chapter_pages(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        ReserveChapterPagesInstr {
            chapter_id: "chapter-1".into(),
            pages: vec![PageImageInstr {
                page_id: Some("page-1".into()),
                image_hash: ImageHash::new([0; 32]),
                new_byte_len: None,
                ext: ImageExt::Png,
            }],
        },
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(reserved.pages[0].slot.is_none());

    assert_eq!(snapshot.pages[0].image_version, Some(2));

    assert_ne!(snapshot.pages[0].is_image_uploaded, Some(true));

    assert!(
        snapshot
            .prom_records
            .iter()
            .all(|record| !matches!(record.payload(), TaskPayload::Image(_)))
    );
}

#[tokio::test]
async fn reserve_chapter_pages_resigns_pending_page_with_new_byte_len() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    mock.seed_page(page(
        "page-1",
        0,
        Some("page/chapter_chapter-1/page-1-2.png"),
        false,
        2,
    ));

    let reserved = reserve_chapter_pages(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        ReserveChapterPagesInstr {
            chapter_id: "chapter-1".into(),
            pages: vec![PageImageInstr {
                page_id: Some("page-1".into()),
                image_hash: ImageHash::new([0; 32]),
                new_byte_len: Some(4096),
                ext: ImageExt::Png,
            }],
        },
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(reserved.pages[0].slot.as_ref().unwrap().image_version, 2);

    assert_eq!(snapshot.pages[0].image_version, Some(2));

    assert_ne!(snapshot.pages[0].is_image_uploaded, Some(true));

    assert!(snapshot.prom_records.iter().any(|record| {
        matches!(
            record.payload(),
            TaskPayload::Image(ImagePayload::CheckUpload {
                resource_id,
                object_key,
                version,
                ..
            }) if resource_id == "page-1"
                && object_key == "page/chapter_chapter-1/page-1-2.png"
                && version == 2
        )
    }));
}

#[tokio::test]
async fn reserve_chapter_pages_rejects_new_page_without_new_byte_len() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let err = reserve_chapter_pages(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        ReserveChapterPagesInstr {
            chapter_id: "chapter-1".into(),
            pages: vec![PageImageInstr {
                page_id: None,
                image_hash: ImageHash::new([1; 32]),
                new_byte_len: None,
                ext: ImageExt::Jpg,
            }],
        },
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(snapshot.pages.is_empty());

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn reserve_chapter_pages_rejects_replacement_without_new_byte_len() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    mock.seed_page(page(
        "page-1",
        0,
        Some("page/chapter_chapter-1/page-1-2.png"),
        true,
        2,
    ));

    let err = reserve_chapter_pages(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        ReserveChapterPagesInstr {
            chapter_id: "chapter-1".into(),
            pages: vec![PageImageInstr {
                page_id: Some("page-1".into()),
                image_hash: ImageHash::new([1; 32]),
                new_byte_len: None,
                ext: ImageExt::Jpg,
            }],
        },
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert_eq!(snapshot.pages[0].image_hash, Some(ImageHash::new([0; 32])));

    assert_eq!(snapshot.pages[0].image_version, Some(2));

    assert_eq!(snapshot.pages[0].is_image_uploaded, Some(true));

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn reserve_chapter_pages_replaces_explicit_image_and_deletes_old_key() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let mut existing_page_info = page("page-1", 0, Some("old.png"), true, 7);

    existing_page_info.total_unit_count = 4;

    mock.seed_page(existing_page_info);

    let reserved = reserve_chapter_pages(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        ReserveChapterPagesInstr {
            chapter_id: "chapter-1".into(),
            pages: vec![PageImageInstr {
                page_id: Some("page-1".into()),
                image_hash: ImageHash::new([1; 32]),
                new_byte_len: Some(8192),
                ext: ImageExt::Jpg,
            }],
        },
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages[0].id, "page-1");

    assert_eq!(snapshot.pages[0].image_version, Some(8));

    assert_eq!(snapshot.pages[0].total_unit_count, 4);

    assert_eq!(snapshot.chapters[0].total_unit_count, 4);

    assert_eq!(reserved.pages[0].slot.as_ref().unwrap().image_version, 8);

    assert!(snapshot.prom_records.iter().any(|record| {
        matches!(
            record.payload(),
            TaskPayload::Image(ImagePayload::Delete { object_key })
                if object_key == "old.png"
        )
    }));
}

#[tokio::test]
async fn reserve_chapter_pages_keeps_raw_pending_when_uploads_are_missing() {
    //
    let mock = Mock::new().with_image_head_absent();

    seed_scope(&mock);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::RAW_PROVIDER),
    ));

    let reserved = reserve_chapter_pages(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        ReserveChapterPagesInstr {
            chapter_id: "chapter-1".into(),
            pages: vec![
                PageImageInstr {
                    page_id: None,
                    image_hash: ImageHash::new([0u8; 32]),
                    new_byte_len: Some(4096),
                    ext: ImageExt::Png,
                },
                PageImageInstr {
                    page_id: None,
                    image_hash: ImageHash::new([0u8; 32]),
                    new_byte_len: Some(4096),
                    ext: ImageExt::Png,
                },
            ],
        },
    )
    .await;

    assert!(reserved.is_ok());

    process_pending(&mock).await.ok().unwrap();

    let snapshot = mock.snapshot();

    assert!(
        snapshot
            .pages
            .iter()
            .all(|page_info| page_info.is_image_uploaded != Some(true))
    );

    assert!(
        snapshot.chapters[0]
            .stages
            .has_phase(Stage::RawProvide, StagePhase::Pending)
    );
}

#[tokio::test]
async fn reserve_chapter_pages_rejects_invalid_count() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    let err = reserve_chapter_pages(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        ReserveChapterPagesInstr {
            chapter_id: "chapter-1".into(),
            pages: Vec::new(),
        },
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(snapshot.pages.is_empty());

    assert!(snapshot.prom_records.is_empty());
}
