#![allow(
    clippy::unwrap_used,
    reason = "Typed database fixtures and assertions fail immediately on violated invariants"
)]

//! Composite manifest reorder, object cleanup and rollback integration.

use std::collections::BTreeMap;

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Nucl as _, OperRun as _, OperStep as _};
use time::OffsetDateTime;
use url::Url;

use poprako_obj_dept::key::{KeyMap as _, ObjGen};
use poprako_obj_dept::model::slot::{ObjDeptPoolSlot, ObjSlotSpec};
use poprako_obj_dept::model::url::{ObjUrlSpec, ObjUrls};
use poprako_obj_dept::oper::{
    DeleteObjs, GenObjSlot, ListObjMetas, MarkObjUploaded,
};
use poprako_obj_dept::pool::{ObjDeptPool, ObjDeptPoolView};
use poprako_obj_dept::rest::ObjDeptRest;

use crate::model::write::issue::{ChapterIssuesRepl, IssueEntry};
use crate::model::write::page_artwork::{PageArtworkEntry, PageArtworkPatch};
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::PageArtworkImage;
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::issue::{ListIssueInfos, ReplaceChapterIssues};
use crate::part::repo::oper::page_artwork::{
    ListPageArtworkInfos, ReplacePageArtworkManifest, UpdatePageArtworkInfo,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::obj_dept::{NormObjDept, RdbObjDeptProm};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::schema::t_obj_prom_task;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::{BaseError, accept};
use crate::shared::test_rdb::start;
use crate::value::image::ImageExt;
use crate::value::page_artwork::PageArtworkImageKey;

#[derive(Clone)]
struct Pool;

impl ObjDeptPoolView for Pool {
    async fn gen_urls(
        &self,
        _key: &str,
        _spec: ObjUrlSpec,
    ) -> ObjDeptRest<ObjUrls> {
        Ok(ObjUrls {
            origin_url: None,
            optimized_url: None,
            thumbnail_url: None,
        })
    }

    async fn has(&self, _key: &str) -> ObjDeptRest<bool> {
        Ok(true)
    }
}

impl ObjDeptPool for Pool {
    async fn gen_slot(
        &self,
        _key: &str,
        _content_type: &str,
        _byte_len: u64,
    ) -> ObjDeptRest<ObjDeptPoolSlot> {
        Ok(ObjDeptPoolSlot {
            url: Url::parse("https://example.test/upload").unwrap(),
            headers: BTreeMap::new(),
            expires_at: OffsetDateTime::now_utc() + time::Duration::minutes(10),
        })
    }

    async fn del(&self, _key: &str) -> ObjDeptRest<()> {
        Ok(())
    }
}

// composite_storage(ReplacePageArtworkManifest, UpdatePageArtworkInfo, DeleteObjs)(positive,negative): reorder preserves identities; targeted metadata writes preserve siblings; failed deletion rolls back metadata, issues and object cleanup.
#[tokio::test]
#[serial_test::serial(repo_rdb)]
async fn composite_manifest_reorders_and_deletes_with_transactional_objects() {
    let test_rdb = start().await;

    let core = test_rdb.core();

    let fixture = test_shared::seed_chapter(&core, "rdb-test-composite-").await;

    let chapter_id = &fixture.chapter_entry.id;

    let repo = HybRepo::new(core.clone());

    let nucl = RdbNucl::<ReptRead>::new(core.clone());

    let dept =
        NormObjDept::new(core.clone(), Pool, RdbObjDeptProm::new(core.clone()));

    let mut entries = vec![
        PageArtworkEntry {
            id: "composite-a".into(),
            chapter_id: chapter_id.clone(),
            index: 0,
            raw_ident: Some("same.psd".into()),
        },
        PageArtworkEntry {
            id: "composite-b".into(),
            chapter_id: chapter_id.clone(),
            index: 1,
            raw_ident: Some("same.psd".into()),
        },
    ];

    let dom = PageArtworkImageKey {
        chapter_id: chapter_id.clone(),
        page_artwork_id: "composite-a".into(),
        ext: ImageExt::Png,
    };

    let key = PageArtworkImage::forward(&dom, 1);

    assert_eq!(
        PageArtworkImage::reverse(&key).unwrap().0.page_artwork_id,
        "composite-a"
    );

    assert!(
        PageArtworkImage::reverse(&"page_artwork/chapter_/a-1.png".into())
            .is_err()
    );

    let spec = ObjSlotSpec {
        dom,
        hash: &[1; 32],
        content_type: "image/png",
        byte_len: 100,
    };

    nucl.coord(async |context| {
        GetChapterInfoExcluded {
            id: chapter_id,
            incls: &[],
        }
        .step_on(&repo, context)
        .await?;

        ReplacePageArtworkManifest {
            chapter_id,
            entries: &entries,
        }
        .step_on(&repo, context)
        .await?;

        let slot = GenObjSlot::<PageArtworkImage>::new(&spec)
            .step_on(&dept, context)
            .await
            .map_err(BaseError::from)?
            .unwrap();

        assert!(
            MarkObjUploaded::<PageArtworkImage>::new(&ObjGen {
                id: "composite-a".into(),
                ver: slot.key.ver
            })
            .step_on(&dept, context)
            .await
            .map_err(BaseError::from)?
        );

        let issues = [IssueEntry {
            id: "composite-issue".into(),
            page_artwork_id: "composite-a".into(),
            index: 0,
            variant: "custom".into(),
            layer_name: None,
            rect: None,
            note: "review".into(),
        }];

        let repl = ChapterIssuesRepl {
            chapter_id,
            entries: &issues,
        };

        ReplaceChapterIssues { repl: &repl }
            .step_on(&repo, context)
            .await?;

        accept(())
    })
    .await
    .unwrap();

    entries.first_mut().unwrap().index = 1;

    entries.get_mut(1).unwrap().index = 0;

    nucl.coord(async |context| {
        GetChapterInfoExcluded {
            id: chapter_id,
            incls: &[],
        }
        .step_on(&repo, context)
        .await?;

        ReplacePageArtworkManifest {
            chapter_id,
            entries: &entries,
        }
        .step_on(&repo, context)
        .await
    })
    .await
    .unwrap();

    assert_eq!(
        ListPageArtworkInfos { chapter_id }
            .run_on(&repo)
            .await
            .unwrap()
            .as_slice()
            .first()
            .unwrap()
            .id,
        "composite-b"
    );

    assert_eq!(
        ListIssueInfos { chapter_id }
            .run_on(&repo)
            .await
            .unwrap()
            .len(),
        1
    );

    let before_update = ListPageArtworkInfos { chapter_id }
        .run_on(&repo)
        .await
        .unwrap();

    let page_artwork_patch = PageArtworkPatch {
        id: "composite-a".into(),
        raw_ident: Some("renamed.psd".into()),
    };

    nucl.coord(async |context| {
        UpdatePageArtworkInfo {
            update: &page_artwork_patch,
        }
        .step_on(&repo, context)
        .await
    })
    .await
    .unwrap();

    let after_update = ListPageArtworkInfos { chapter_id }
        .run_on(&repo)
        .await
        .unwrap();

    assert_eq!(
        after_update.as_slice().first(),
        before_update.as_slice().first()
    );

    assert_eq!(
        after_update.get(1).unwrap().raw_ident.as_deref(),
        Some("renamed.psd")
    );

    assert_eq!(
        after_update.get(1).unwrap().created_at,
        before_update.get(1).unwrap().created_at
    );

    let failure = nucl
        .coord(async |context| {
            let page_artwork_patch = PageArtworkPatch {
                id: "composite-a".into(),
                raw_ident: None,
            };

            UpdatePageArtworkInfo {
                update: &page_artwork_patch,
            }
            .step_on(&repo, context)
            .await?;

            DeleteObjs::<PageArtworkImage>::new(&["composite-a".into()])
                .step_on(&dept, context)
                .await
                .map_err(BaseError::from)?;

            ReplacePageArtworkManifest {
                chapter_id,
                entries: &[],
            }
            .step_on(&repo, context)
            .await?;

            Err::<(), _>(BaseError::Unrecoverable {
                msg: "rollback composite deletion".into(),
            })
        })
        .await;

    assert!(failure.is_err());

    assert_eq!(
        ListPageArtworkInfos { chapter_id }
            .run_on(&repo)
            .await
            .unwrap(),
        after_update
    );

    assert_eq!(
        ListIssueInfos { chapter_id }
            .run_on(&repo)
            .await
            .unwrap()
            .len(),
        1
    );

    assert_eq!(
        ListPageArtworkInfos { chapter_id }
            .run_on(&repo)
            .await
            .unwrap()
            .len(),
        2
    );

    assert!(
        ListObjMetas::<PageArtworkImage>::new(&["composite-a"])
            .run_on(&dept)
            .await
            .unwrap()
            .get("composite-a")
            .unwrap()
            .is_avail
    );

    nucl.coord(async |context| {
        DeleteObjs::<PageArtworkImage>::new(&["composite-a".into()])
            .step_on(&dept, context)
            .await
            .map_err(BaseError::from)?;

        ReplacePageArtworkManifest {
            chapter_id,
            entries: &[],
        }
        .step_on(&repo, context)
        .await
    })
    .await
    .unwrap();

    assert!(
        ListIssueInfos { chapter_id }
            .run_on(&repo)
            .await
            .unwrap()
            .is_empty()
    );

    let mut conn = core.get().await.unwrap();

    let count = t_obj_prom_task::table
        .filter(t_obj_prom_task::f_topic.eq("page_artwork_image"))
        .count()
        .get_result::<i64>(&mut *conn)
        .await
        .unwrap();

    assert!(count > 0);

    drop(conn);

    test_shared::cleanup(&core, "rdb-test-composite-")
        .await
        .unwrap();
}
