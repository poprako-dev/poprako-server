#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::*;
use crate::value::business_id::{
    ChapterId, ComicId, PageSourceId, TeamId, UserId,
};

use poprako_obj_dept::model::record::{
    ObjKeyRecord as ObjKey, ObjMetaRecord as ObjMeta,
};

use poprako_orchestra::{Nucl as _, OperRun as _, OperStep as _};

use poprako_obj_dept::key::{ObjGen, ObjKind};
use poprako_obj_dept::model::slot::ObjSlotSpec;
use poprako_obj_dept::model::url::ObjUrlSpec;
use poprako_obj_dept::oper::{
    ClearObjs, DeleteObjs, GenObjUrls, MarkObjUploaded,
};

use crate::part::obj_dept::{
    ComicCover, PageSourceImage, TeamAvatar, UserAvatar,
};
use crate::value::image::{
    ComicCoverKey, ImageExt, PageSourceImageKey, TeamAvatarKey, UserAvatarKey,
};

fn page_source_dom(page_source_id: &str) -> PageSourceImageKey {
    PageSourceImageKey {
        chapter_id: ChapterId::new("chapter--00000000001").unwrap(),
        page_source_id: PageSourceId::new(page_source_id).unwrap(),
        ext: ImageExt::Png,
    }
}

#[test]
fn key_maps_preserve_the_business_physical_key_contract() {
    let page_source_key = PageSourceImage::forward(
        &page_source_dom("page_source--00000000001"),
        7,
    );

    let user_key = UserAvatar::forward(
        &UserAvatarKey {
            user_id: UserId::new("user--00000000001").unwrap(),
            ext: ImageExt::Jpg,
        },
        8,
    );

    let team_key = TeamAvatar::forward(
        &TeamAvatarKey {
            team_id: TeamId::new("team--00000000001").unwrap(),
            ext: ImageExt::Webp,
        },
        9,
    );

    let comic_key = ComicCover::forward(
        &ComicCoverKey {
            comic_id: ComicId::new("comic--00000000001").unwrap(),
            ext: ImageExt::Avif,
        },
        10,
    );

    assert_eq!(
        page_source_key.as_str(),
        "page_source/chapter--00000000001/page_source--00000000001/7.png"
    );

    assert_eq!(user_key.as_str(), "user_avatar/user--00000000001/8.jpg");

    assert_eq!(team_key.as_str(), "team_avatar/team--00000000001/9.webp");

    assert_eq!(comic_key.as_str(), "comic_cover/comic--00000000001/10.avif");
}

#[test]
fn origin_only_spec_omits_image_renditions() {
    let meta = ObjMeta {
        key: ObjKey {
            id: String::from("page_source--00000000001"),
            ver: 1,
            domain: serde_json::json!({
                "chapter_id": "chapter--00000000001",
                "page_source_id": "page_source--00000000001",
                "ext": "png",
            }),
        },
        is_avail: true,
        hash: vec![1; 32],
        ext: String::from("png"),
    };

    let obj_url_spec = ObjUrlSpec::default().with_origin();

    let urls = gen_urls(
        Some(&meta.decode::<PageSourceImage>().unwrap()),
        obj_url_spec,
        true,
    )
    .unwrap()
    .unwrap();

    assert!(urls.origin_url.is_some());

    assert!(urls.optimized_url.is_none());

    assert!(urls.thumbnail_url.is_none());
}

#[test]
fn selected_image_renditions_are_generated() {
    let meta = ObjMeta {
        key: ObjKey {
            id: String::from("page_source--00000000001"),
            ver: 1,
            domain: serde_json::json!({
                "chapter_id": "chapter--00000000001",
                "page_source_id": "page_source--00000000001",
                "ext": "png",
            }),
        },
        is_avail: true,
        hash: vec![1; 32],
        ext: String::from("png"),
    };

    let obj_url_spec = ObjUrlSpec::default().with_optimized().with_thumbnail();

    let urls = gen_urls(
        Some(&meta.decode::<PageSourceImage>().unwrap()),
        obj_url_spec,
        true,
    )
    .unwrap()
    .unwrap();

    assert!(urls.origin_url.is_none());

    assert!(urls.optimized_url.is_some());

    assert!(urls.thumbnail_url.is_some());
}

#[tokio::test]
async fn mock_operation_rejects_empty_url_spec_without_metadata() {
    let mock = Mock::new();

    let metas = HashMap::new();

    let error =
        GenObjUrls::<PageSourceImage>::new(&metas, ObjUrlSpec::default())
            .run_on(&mock)
            .await
            .err()
            .expect("URL generation must reject an empty URL spec");

    assert!(matches!(error, ObjDeptError::Invalid { .. }));
}

#[tokio::test]
async fn mock_operation_can_disable_thumbnails() {
    let mock = Mock::new().with_obj_thumbnail_disabled();
    let meta = ObjMeta {
        key: ObjKey {
            id: String::from("page_source--00000000001"),
            ver: 1,
            domain: serde_json::json!({
                "chapter_id": "chapter--00000000001",
                "page_source_id": "page_source--00000000001",
                "ext": "png",
            }),
        },
        is_avail: true,
        hash: vec![1; 32],
        ext: String::from("png"),
    };
    let meta = meta.decode::<PageSourceImage>().unwrap();

    let metas = HashMap::from([(meta.key.id().clone(), meta)]);

    let obj_url_spec = ObjUrlSpec::default()
        .with_origin()
        .with_optimized()
        .with_thumbnail();

    let urls = GenObjUrls::<PageSourceImage>::new(&metas, obj_url_spec)
        .run_on(&mock)
        .await
        .unwrap();

    assert!(matches!(
        urls.get("page_source--00000000001"),
        Some(ObjUrls {
            thumbnail_url: None,
            ..
        }),
    ));

    assert!(matches!(
        urls.get("page_source--00000000001"),
        Some(ObjUrls {
            optimized_url: Some(_),
            ..
        }),
    ));
}

#[tokio::test]
async fn upload_mark_is_exact_current_and_idempotent() {
    let mock = Mock::new();
    let key = ObjKey {
        id: String::from("page_source--00000000001"),
        ver: 4,
        domain: serde_json::json!({
            "chapter_id": "chapter--00000000001",
            "page_source_id": "page_source--00000000001",
            "ext": "png",
        }),
    };
    let meta = ObjMeta {
        key: key.clone(),
        is_avail: false,
        hash: vec![4; 32],
        ext: String::from("png"),
    };

    mock.state
        .lock()
        .unwrap()
        .objs
        .entry("page_source_image")
        .or_default()
        .insert(
            key.id.clone(),
            MockObjRecord {
                version: key.ver,
                meta: Some(meta),
            },
        );

    let generation = ObjGen::<PageSourceImage>::new(
        PageSourceImage::ids(std::slice::from_ref(&key.id))
            .unwrap()
            .remove(0),
        key.ver,
    );

    let first = MarkObjUploaded::<PageSourceImage>::new(&generation)
        .run_on(&mock)
        .await
        .unwrap();
    let second = MarkObjUploaded::<PageSourceImage>::new(&generation)
        .run_on(&mock)
        .await
        .unwrap();
    let stale_key = ObjGen::<PageSourceImage>::new(
        PageSourceImage::ids(std::slice::from_ref(&key.id))
            .unwrap()
            .remove(0),
        3,
    );
    let stale = MarkObjUploaded::<PageSourceImage>::new(&stale_key)
        .run_on(&mock)
        .await
        .unwrap();
    let missing_key = ObjGen::<PageSourceImage>::new(
        PageSourceImage::ids(&[String::from("page_source--00000000002")])
            .unwrap()
            .remove(0),
        1,
    );
    let missing = MarkObjUploaded::<PageSourceImage>::new(&missing_key)
        .run_on(&mock)
        .await
        .unwrap();

    assert!(first);

    assert!(second);

    assert!(!stale);

    assert!(!missing);

    let snapshot = mock.snapshot();
    let uploaded = snapshot
        .objs
        .get("page_source_image")
        .and_then(|objs| objs.get("page_source--00000000001"))
        .and_then(|record| record.meta.as_ref())
        .is_some_and(|meta| meta.is_avail);

    assert!(uploaded);

    let detached_prepared = {
        let mut mock_state = mock.state.lock().unwrap();

        match mock_state
            .objs
            .get_mut("page_source_image")
            .and_then(|objs| objs.get_mut("page_source--00000000001"))
        {
            Some(record) => {
                record.meta = None;

                true
            }
            None => false,
        }
    };

    assert!(detached_prepared);

    let detached = MarkObjUploaded::<PageSourceImage>::new(&generation)
        .run_on(&mock)
        .await
        .unwrap();

    assert!(!detached);
}

#[tokio::test]
async fn slot_and_delete_defer_check_and_delete_debt() {
    let mock = Mock::new();

    let obj_dept = mock.clone();

    mock.coord(async move |context| {
        //
        let obj_spec = ObjSlotSpec {
            dom: page_source_dom("page_source--00000000001"),
            hash: &[1; 32],
            content_type: "image/png",
            byte_len: 1024,
        };

        GenObjSlot::<PageSourceImage>::new(&obj_spec)
            .step_on(&obj_dept, context)
            .await?;

        let ids = vec![String::from("page_source--00000000001")];

        DeleteObjs::<PageSourceImage>::new(
            &PageSourceImage::ids(&PageSourceImage::ids(&ids).unwrap())
                .unwrap(),
        )
        .step_on(&obj_dept, context)
        .await?;

        Ok::<(), ObjDeptError>(())
    })
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(
        snapshot
            .objs
            .get("page_source_image")
            .is_none_or(HashMap::is_empty)
    );

    assert_eq!(snapshot.obj_tasks.len(), 2);

    assert!(matches!(
        snapshot.obj_tasks.first().unwrap().1,
        ObjTask::Check { .. }
    ));

    assert!(matches!(
        snapshot.obj_tasks.get(1).unwrap().1,
        ObjTask::Delete { .. }
    ));
}

#[tokio::test]
async fn allocs_retain_generations_for_new_pending_and_avail_content() {
    let mock = Mock::new();

    let specs = [
        "page_source--00000000001",
        "page_source--00000000002",
        "page_source--00000000003",
    ]
    .map(|id| ObjSlotSpec {
        dom: page_source_dom(id),
        hash: &[1; 32],
        content_type: "image/png",
        byte_len: 1024,
    });

    let first = mock
        .coord(async |context| {
            GenObjSlot::<PageSourceImage>::new(&specs[0])
                .step_on(&mock, context)
                .await
        })
        .await
        .unwrap();

    assert_eq!(first.key().ver(), 1);

    let first_key = first.key().clone();

    let first_slot = first.into_slot().unwrap();

    assert_eq!(first_slot.key, first_key);

    assert!(first_slot.url.path().ends_with(first_key.image().as_str()));

    mock.coord(async |context| {
        let resumed = GenObjSlot::<PageSourceImage>::new(&specs[0])
            .step_on(&mock, context)
            .await?;

        assert_eq!(resumed.key(), &first_key);

        assert_eq!(resumed.into_slot().unwrap().key, first_key);

        MarkObjUploaded::<PageSourceImage>::new(
            &ObjGen::<PageSourceImage>::new(
                first_key.id().clone(),
                first_key.ver(),
            ),
        )
        .step_on(&mock, context)
        .await?;

        let task_count = context.state.obj_tasks.len();

        let available = GenObjSlot::<PageSourceImage>::new(&specs[0])
            .step_on(&mock, context)
            .await?;

        assert_eq!(available.key(), &first_key);

        assert!(available.into_slot().is_none());

        assert_eq!(context.state.obj_tasks.len(), task_count);

        GenObjSlot::<PageSourceImage>::new(&specs[1])
            .step_on(&mock, context)
            .await?;

        Ok::<(), ObjDeptError>(())
    })
    .await
    .unwrap();

    let before = mock.snapshot();

    let allocs = mock
        .coord(async |context| {
            GenObjSlots::<PageSourceImage>::new(&specs)
                .step_on(&mock, context)
                .await
        })
        .await
        .unwrap();

    assert_eq!(allocs.len(), specs.len());

    for (id, alloc) in allocs {
        assert_eq!(alloc.key().id(), &id);

        assert_eq!(alloc.key().ver(), 1);

        match alloc.into_slot() {
            None => assert_eq!(id.as_str(), "page_source--00000000001"),
            Some(slot) => {
                assert_ne!(id.as_str(), "page_source--00000000001");

                assert!(slot.url.path().ends_with(slot.key.image().as_str()));
            }
        }
    }

    let after = mock.snapshot();

    assert!(after.objs.get("page_source_image").unwrap().values().all(
        |record| {
            record.version == 1 && record.meta.as_ref().unwrap().key.ver == 1
        }
    ));

    assert_eq!(after.obj_tasks.len(), before.obj_tasks.len() + 2);

    assert!(after.obj_tasks.iter().all(
        |(_, task)| matches!(task, ObjTask::Check { key } if key.ver == 1)
    ));
}

#[tokio::test]
async fn clear_allows_replacement_without_reusing_a_generation() {
    let mock = Mock::new();
    let obj_dept = mock.clone();

    let replacement = mock
        .coord(async move |context| {
            let obj_spec = ObjSlotSpec {
                dom: page_source_dom("page_source--00000000001"),
                hash: &[1; 32],
                content_type: "image/png",
                byte_len: 1024,
            };

            GenObjSlot::<PageSourceImage>::new(&obj_spec)
                .step_on(&obj_dept, context)
                .await?;

            let ids = vec![String::from("page_source--00000000001")];

            ClearObjs::<PageSourceImage>::new(
                &PageSourceImage::ids(&PageSourceImage::ids(&ids).unwrap())
                    .unwrap(),
            )
            .step_on(&obj_dept, context)
            .await?;

            GenObjSlot::<PageSourceImage>::new(&obj_spec)
                .step_on(&obj_dept, context)
                .await
        })
        .await
        .unwrap();

    assert_eq!(replacement.into_slot().unwrap().key.ver(), 2);
}

#[tokio::test]
async fn batch_slots_reject_duplicate_ids_before_mutation() {
    let mock = Mock::new();
    let obj_dept = mock.clone();

    let result = mock
        .coord(async move |context| {
            let specs = [
                ObjSlotSpec {
                    dom: page_source_dom("page_source--00000000001"),
                    hash: &[1; 32],
                    content_type: "image/png",
                    byte_len: 1024,
                },
                ObjSlotSpec {
                    dom: page_source_dom("page_source--00000000001"),
                    hash: &[2; 32],
                    content_type: "image/png",
                    byte_len: 2048,
                },
            ];

            GenObjSlots::<PageSourceImage>::new(&specs)
                .step_on(&obj_dept, context)
                .await
        })
        .await;

    assert!(matches!(
        result,
        Err(poprako_orchestra::nucl::Error::Step(
            ObjDeptError::Invalid { .. }
        )),
    ));

    let snapshot = mock.snapshot();

    assert!(!snapshot.objs.contains_key("page_source_image"));

    assert!(snapshot.obj_tasks.is_empty());
}

// artwork_key_mapping(ChapterArtwork)(positive): arbitrary safe suffixes retain their exact physical representation.
#[test]
fn artwork_key_mapping_preserves_safe_suffixes() {
    use crate::part::obj_dept::ChapterArtwork;
    use crate::value::artwork::{ArtworkExt, ChapterArtworkKey};
    use ChapterId;

    let artwork_key = ChapterArtworkKey {
        chapter_id: ChapterId::new("chapter--00000000123").unwrap(),
        ext: ArtworkExt::new("PSD").unwrap(),
    };

    assert_eq!(
        ChapterArtwork::forward(&artwork_key, 3).as_str(),
        "chapter_artwork/chapter--00000000123/3.PSD"
    );
}
