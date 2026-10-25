//! Versioned page editing and chapter-wide category behavior.

use super::{input, review_mock, submit, token};
use crate::data::instr::issue_edit::{
    IssueEditEntryInstr, RenameIssueEditVariantInstr, SavePageIssuesInstr,
};
use crate::data::val::issue_edit::IssueEditPageVal;
use crate::part_impl::cache::mem_impl::MemCache;
use crate::part_impl::repo::mock_impl::Mock;
use crate::usecase::issue_edit::{
    get_info, list_variant_infos, rename::rename_variant, save::save_infos,
};
use crate::value::role::RoleField;
use uuid::Uuid;

async fn document(mock: &Mock, id: &str) -> IssueEditPageVal {
    get_info((mock, mock, mock), token("user--00000000001"), id.into())
        .await
        .unwrap()
}

fn replacement(
    page: &IssueEditPageVal,
    id: &str,
    note: &str,
) -> SavePageIssuesInstr {
    SavePageIssuesInstr {
        review_key: page.review_key.clone(),
        revision: page.revision.clone(),
        image_ver: page.image_ver,
        issues: vec![IssueEditEntryInstr {
            id: id.starts_with("issue--").then(|| id.to_owned()),
            local_id: (!id.starts_with("issue--")).then(|| id.to_owned()),
            variant: "custom".into(),
            rect: None,
            note: note.into(),
        }],
    }
}

// page_edit(save_infos)(positive): stable IDs, page isolation, replay and empty replacement.
#[tokio::test]
async fn stable_page_save_replays_without_overwriting_newer_edits() {
    let cache = MemCache::default();

    let mock = review_mock(RoleField::REVIEWER);

    submit(&mock, input("sibling")).await.unwrap();

    let baseline = document(&mock, "page_artwork--00000000001").await;

    let id = Uuid::new_v4().to_string();

    let batch = Uuid::new_v4();

    let saved = save_infos(
        (&mock, &mock, &cache, &mock),
        token("user--00000000001"),
        "page_artwork--00000000001".into(),
        batch,
        replacement(&baseline, &id, "first"),
    )
    .await
    .unwrap();

    assert_ne!(saved.page.revision, baseline.revision);

    let latest = save_infos(
        (&mock, &mock, &cache, &mock),
        token("user--00000000001"),
        "page_artwork--00000000001".into(),
        Uuid::new_v4(),
        replacement(
            &saved.page,
            &saved.created_issue_ids.first().unwrap().issue_id,
            "second",
        ),
    )
    .await
    .unwrap();

    let replay = save_infos(
        (&mock, &mock, &cache, &mock),
        token("user--00000000001"),
        "page_artwork--00000000001".into(),
        batch,
        replacement(&baseline, &id, "first"),
    )
    .await
    .unwrap();

    assert_eq!(replay.page.revision, latest.page.revision);

    assert_eq!(
        replay.created_issue_ids.first().unwrap().issue_id,
        saved.created_issue_ids.first().unwrap().issue_id
    );

    assert_eq!(saved.created_issue_ids.first().unwrap().local_id, id);

    assert_eq!(replay.page.issues.first().unwrap().note, "second");

    assert_eq!(
        document(&mock, "page_artwork--00000000000")
            .await
            .issues
            .first()
            .unwrap()
            .note,
        "sibling"
    );

    let mut clear = replacement(&latest.page, &id, "");

    clear.issues.clear();

    let cleared = save_infos(
        (&mock, &mock, &cache, &mock),
        token("user--00000000001"),
        "page_artwork--00000000001".into(),
        Uuid::new_v4(),
        clear,
    )
    .await
    .unwrap();

    assert!(cleared.page.issues.is_empty());
}

// page_edit(save_infos)(negative): stale versions, reused receipts and foreign IDs preserve data.
#[tokio::test]
async fn invalid_save_context_does_not_modify_pages() {
    let cache = MemCache::default();

    let mock = review_mock(RoleField::REVIEWER);

    submit(&mock, input("original")).await.unwrap();

    let baseline = document(&mock, "page_artwork--00000000000").await;

    let id = baseline.issues.first().unwrap().id.clone();

    let batch = Uuid::new_v4();

    save_infos(
        (&mock, &mock, &cache, &mock),
        token("user--00000000001"),
        "page_artwork--00000000000".into(),
        batch,
        replacement(&baseline, &id, "changed"),
    )
    .await
    .unwrap();

    for save_id in [batch, Uuid::new_v4()] {
        assert!(
            save_infos(
                (&mock, &mock, &cache, &mock),
                token("user--00000000001"),
                "page_artwork--00000000000".into(),
                save_id,
                replacement(&baseline, &id, "invalid")
            )
            .await
            .is_err()
        );
    }

    let sibling = document(&mock, "page_artwork--00000000001").await;

    assert!(
        save_infos(
            (&mock, &mock, &cache, &mock),
            token("user--00000000001"),
            "page_artwork--00000000001".into(),
            Uuid::new_v4(),
            replacement(&sibling, &id, "foreign")
        )
        .await
        .is_err()
    );

    assert_eq!(
        document(&mock, "page_artwork--00000000000")
            .await
            .issues
            .first()
            .unwrap()
            .note,
        "changed"
    );

    submit(&mock, input("imported")).await.unwrap();

    assert!(
        save_infos(
            (&mock, &mock, &cache, &mock),
            token("user--00000000001"),
            "page_artwork--00000000000".into(),
            batch,
            replacement(&baseline, &id, "changed")
        )
        .await
        .is_err()
    );
}

// category_edit(rename_variant)(positive): rename all references atomically and invalidate page baselines.
#[tokio::test]
async fn rename_updates_both_pages_and_summary() {
    let cache = MemCache::default();

    let mock = review_mock(RoleField::REVIEWER);

    for page_id in ["page_artwork--00000000000", "page_artwork--00000000001"] {
        let page = document(&mock, page_id).await;

        save_infos(
            (&mock, &mock, &cache, &mock),
            token("user--00000000001"),
            page_id.into(),
            Uuid::new_v4(),
            replacement(&page, &Uuid::new_v4().to_string(), "note"),
        )
        .await
        .unwrap();
    }

    let before = list_variant_infos(
        (&mock, &mock),
        token("user--00000000001"),
        "chapter--00000000001".into(),
    )
    .await
    .unwrap();

    assert_eq!(before.variants.first().unwrap().count, 2);

    let renamed = rename_variant(
        (&mock, &mock, &mock),
        token("user--00000000001"),
        "chapter--00000000001".into(),
        RenameIssueEditVariantInstr {
            review_key: before.review_key,
            revision: before.revision,
            from: "custom".into(),
            to: "new category".into(),
        },
    )
    .await
    .unwrap();

    assert_eq!(renamed.pages.len(), 2);

    assert_eq!(
        renamed.summary.variants.first().unwrap().variant,
        "new category"
    );

    assert_eq!(renamed.summary.variants.first().unwrap().count, 2);

    assert!(
        renamed
            .pages
            .iter()
            .all(|page| page.issues.first().unwrap().variant == "new category")
    );
}

// page_edit(save_infos)(negative): revoked assignment and transaction failure cannot write or replay.
#[tokio::test]
async fn permission_and_rollback_precede_receipt_recording() {
    let cache = MemCache::default();

    let mock = review_mock(RoleField::REVIEWER);

    let baseline = document(&mock, "page_artwork--00000000000").await;

    let id = Uuid::new_v4().to_string();

    let batch = Uuid::new_v4();

    let comics = mock.snapshot().comics;

    mock.state.lock().unwrap().comics.clear();

    assert!(
        save_infos(
            (&mock, &mock, &cache, &mock),
            token("user--00000000001"),
            "page_artwork--00000000000".into(),
            batch,
            replacement(&baseline, &id, "retry")
        )
        .await
        .is_err()
    );

    assert!(mock.snapshot().issues.is_empty());

    mock.state.lock().unwrap().comics = comics;

    save_infos(
        (&mock, &mock, &cache, &mock),
        token("user--00000000001"),
        "page_artwork--00000000000".into(),
        batch,
        replacement(&baseline, &id, "retry"),
    )
    .await
    .unwrap();

    mock.state.lock().unwrap().assignments.clear();

    assert!(
        save_infos(
            (&mock, &mock, &cache, &mock),
            token("user--00000000001"),
            "page_artwork--00000000000".into(),
            batch,
            replacement(&baseline, &id, "retry")
        )
        .await
        .is_err()
    );
}

// page_edit(save_infos)(negative): a failed commit cannot retain a fingerprint or block a corrected retry.
#[tokio::test]
async fn failed_commit_does_not_publish_receipt() {
    let cache = MemCache::default();

    let mock = review_mock(RoleField::REVIEWER);

    let baseline = document(&mock, "page_artwork--00000000000").await;

    let id = Uuid::new_v4().to_string();

    let batch = Uuid::new_v4();

    mock.set_commit_failure(true);

    assert!(
        save_infos(
            (&mock, &mock, &cache, &mock),
            token("user--00000000001"),
            "page_artwork--00000000000".into(),
            batch,
            replacement(&baseline, &id, "failed")
        )
        .await
        .is_err()
    );

    assert!(mock.snapshot().issues.is_empty());

    mock.set_commit_failure(false);

    let cloned_cache = cache.clone();

    let (first, replay) = tokio::join!(
        save_infos(
            (&mock, &mock, &cache, &mock),
            token("user--00000000001"),
            "page_artwork--00000000000".into(),
            batch,
            replacement(&baseline, &id, "retry"),
        ),
        save_infos(
            (&mock, &mock, &cloned_cache, &mock),
            token("user--00000000001"),
            "page_artwork--00000000000".into(),
            batch,
            replacement(&baseline, &id, "retry"),
        ),
    );

    let first = first.unwrap();

    let replay = replay.unwrap();

    assert_eq!(first.page.revision, replay.page.revision);

    assert_eq!(first.page.issues.first().unwrap().note, "retry");

    assert_eq!(mock.snapshot().issues.len(), 1);
}

// save_infos(negative): new identities require unique local IDs and existing identities must already belong to this page.
#[tokio::test]
async fn ambiguous_or_unknown_issue_identities_are_rejected() {
    let cache = MemCache::default();

    let mock = review_mock(RoleField::REVIEWER);

    let baseline = document(&mock, "page_artwork--00000000000").await;

    for (id, local_id) in [
        (None, None),
        (Some("issue--00000000001"), Some("draft")),
        (Some("issue--00000000001"), None),
        (Some("unit--00000000001"), None),
        (None, Some("")),
    ] {
        let mut instr = replacement(&baseline, "draft", "note");

        let issue = instr.issues.first_mut().unwrap();

        issue.id = id.map(str::to_owned);

        issue.local_id = local_id.map(str::to_owned);

        assert!(
            save_infos(
                (&mock, &mock, &cache, &mock),
                token("user--00000000001"),
                "page_artwork--00000000000".into(),
                Uuid::new_v4(),
                instr
            )
            .await
            .is_err()
        );

        assert!(mock.snapshot().issues.is_empty());
    }

    let mut instr = replacement(&baseline, "duplicate", "note");

    instr.issues.push(IssueEditEntryInstr {
        id: None,
        local_id: Some("duplicate".into()),
        variant: "custom".into(),
        rect: None,
        note: "other".into(),
    });

    assert!(
        save_infos(
            (&mock, &mock, &cache, &mock),
            token("user--00000000001"),
            "page_artwork--00000000000".into(),
            Uuid::new_v4(),
            instr
        )
        .await
        .is_err()
    );

    assert!(mock.snapshot().issues.is_empty());
}
