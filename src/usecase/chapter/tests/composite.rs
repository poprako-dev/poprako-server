#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately on violated invariants"
)]

use time::OffsetDateTime;

use poprako_obj_dept::model::record::ObjKeyRecord as ObjKey;
use poprako_obj_dept::model::record::ObjMetaRecord as ObjMeta;
use poprako_obj_dept::model::task::ObjTask;

use super::{Mock, RoleField, RoleMask, chapter, seed_scope, token};

use crate::model::read::proj::issue::IssueInfo;
use crate::model::read::proj::page_artwork::PageArtworkInfo;
use crate::part_impl::repo::mock_impl::MockObjRecord;
use crate::usecase::chapter::delete as chapter_delete_usecase;

// Builds the current-format artwork object fixture.
fn artwork_key() -> ObjKey {
    ObjKey {
        id: "page_artwork--00000000001".into(),
        ver: 1,
        domain: serde_json::json!({
            "chapter_id": "chapter--00000000001",
            "page_artwork_id": "page_artwork--00000000001",
            "ext": "png",
        }),
    }
}

// delete(delete)(positive,negative): direct Chapter deletion cleans independent composite objects and issues; failed cleanup preserves all business state.
#[tokio::test]
async fn chapter_delete_cleans_composites_without_source_page_sources_atomically()
 {
    for fails in [false, true] {
        let mut mock = Mock::new();

        if fails {
            mock = mock.with_obj_delete_failure();
        }

        seed_scope(
            &mock,
            "user--00000000001",
            RoleMask::from(RoleField::ADMIN),
        );

        mock.seed_chapter(chapter(
            "chapter--00000000001",
            "comic--00000000001",
            0,
            false,
        ));

        let time = OffsetDateTime::now_utc();

        let key = artwork_key();

        {
            let mut state = mock.state.lock().unwrap();

            state.page_artworks.push(PageArtworkInfo {
                id: "page_artwork--00000000001".into(),
                chapter_id: "chapter--00000000001".into(),
                index: 0,
                raw_ident: Some("tail.psd".into()),
                created_at: time,
                updated_at: time,
            });

            state.issues.push(IssueInfo {
                id: "issue--00000000001".into(),
                page_artwork_id: "page_artwork--00000000001".into(),
                index: 0,
                variant: "custom".into(),
                rect: None,
                note: "fix the tail page".into(),
            });

            state.objs.entry("page_artwork_image").or_default().insert(
                "page_artwork--00000000001".into(),
                MockObjRecord {
                    version: 1,
                    meta: Some(ObjMeta {
                        key: key.clone(),
                        hash: vec![1; 32],
                        ext: "png".into(),
                        is_avail: true,
                    }),
                },
            );
        }

        let before = mock.snapshot();

        assert!(before.pages.is_empty());

        let result = chapter_delete_usecase::delete(
            (&mock, &mock, &mock),
            token("user--00000000001"),
            "chapter--00000000001".into(),
        )
        .await;

        let after = mock.snapshot();

        let image_records = after.objs.get("page_artwork_image").unwrap();

        if fails {
            assert!(result.is_err());

            assert_eq!(after.chapters.len(), before.chapters.len());

            assert_eq!(
                after.chapters.first().unwrap().id,
                "chapter--00000000001"
            );

            assert_eq!(after.page_artworks, before.page_artworks);

            assert_eq!(after.issues, before.issues);

            assert!(after.obj_tasks.is_empty());

            assert!(
                image_records
                    .get("page_artwork--00000000001")
                    .unwrap()
                    .meta
                    .is_some()
            );

            continue;
        }

        result.unwrap();

        assert!(after.chapters.is_empty());

        assert!(after.page_artworks.is_empty());

        assert!(after.issues.is_empty());

        assert!(image_records.is_empty());

        assert!(after.obj_tasks.iter().any(|(topic, task)| {
            *topic == "page_artwork_image"
                && matches!(
                    task,
                    ObjTask::Delete { key: deleted_key }
                        if deleted_key == &key
                )
        }));

        assert_eq!(after.comics.first().unwrap().chapter_count, 1);
    }
}
