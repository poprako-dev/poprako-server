// save_edits(save_tokens)(positive): duplicate tokens skip all Unit and workflow mutations.
// save_edits(save_tokens)(negative): invalid tokens and unauthorized replays are rejected.
// save_edits(save_tokens)(negative): rollback does not retain a successful token.
use super::*;
use crate::config::cache::CacheConfig;
use std::time::{Duration, Instant};

fn batch(save_id: &str, text: &str) -> SavePageUnitEditsInstr {
    let mut instr = save_instr(vec![create("local-save", None, Some(text))]);

    instr.save_id = save_id.into();

    instr
}

// save_edits(save_tokens)(positive): retries do not renew retention and expired tokens allow execution.
#[tokio::test]
async fn replay_does_not_extend_token_lifetime() {
    let (mock, _) = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let config = CacheConfig {
        unit_save_ttl: Duration::from_secs(2),
        ..CacheConfig::default()
    };

    let cache = MemCache::new(config);

    let save_id = uuid::Uuid::new_v4().to_string();

    let original = save_edits(
        (&mock, &mock, &cache),
        token("translator-1"),
        batch(&save_id, "saved"),
    )
    .await
    .unwrap();

    let recorded_at = Instant::now();

    tokio::time::sleep(Duration::from_secs(1)).await;

    let replay = save_edits(
        (&mock, &mock, &cache),
        token("translator-1"),
        batch(&save_id, "saved"),
    )
    .await
    .unwrap();

    assert_eq!(replay, original);

    assert_eq!(mock.snapshot().units.len(), 1);

    tokio::time::sleep_until(
        (recorded_at + Duration::from_millis(2200)).into(),
    )
    .await;

    let repeated = save_edits(
        (&mock, &mock, &cache),
        token("translator-1"),
        batch(&save_id, "saved"),
    )
    .await
    .unwrap();

    assert_ne!(repeated, original);

    assert_eq!(mock.snapshot().units.len(), 2);
}

#[tokio::test]
async fn retained_token_skips_duplicate_and_changed_batches() {
    let (mock, cache) = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let save_id = uuid::Uuid::new_v4();

    let original = save_edits(
        (&mock, &mock, &cache),
        token("translator-1"),
        batch(&save_id.to_string(), "saved"),
    )
    .await
    .unwrap();

    let original_id = mock.snapshot().units.first().unwrap().id.clone();

    let records = mock.snapshot().chapter_workflow_records.len();

    for (encoded_id, text) in [
        (save_id.to_string(), "saved"),
        (save_id.simple().to_string(), "changed"),
    ] {
        let replay = save_edits(
            (&mock, &mock, &cache),
            token("translator-1"),
            batch(&encoded_id, text),
        )
        .await
        .unwrap();

        assert_eq!(replay, original);

        assert_eq!(
            original.created_unit_ids.first().unwrap().local_id,
            "local-save"
        );

        assert_eq!(
            original.created_unit_ids.first().unwrap().unit_id,
            original_id
        );

        assert_eq!(mock.snapshot().units.len(), 1);

        assert_eq!(mock.snapshot().units.first().unwrap().id, original_id);

        assert_eq!(mock.snapshot().pages.first().unwrap().total_unit_count, 1);

        assert_eq!(mock.snapshot().chapter_workflow_records.len(), records);
    }

    mock.state.lock().unwrap().assignments.clear();

    assert!(
        save_edits(
            (&mock, &mock, &cache),
            token("translator-1"),
            batch(&save_id.to_string(), "saved"),
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn invalid_token_leaves_units_unchanged() {
    let (mock, cache) = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let error = save_edits(
        (&mock, &mock, &cache),
        token("translator-1"),
        batch("not-a-uuid", "saved"),
    )
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        BaseError::Expected {
            variant: ExpectedVariant::Args,
            ..
        }
    ));

    assert!(mock.snapshot().units.is_empty());

    assert_eq!(cache.unit_save_token_count(), 0);
}

#[tokio::test]
async fn failed_batch_does_not_consume_save_identity() {
    let (mock, cache) = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let save_id = uuid::Uuid::new_v4().to_string();

    let mut invalid = batch(&save_id, "text");

    invalid.edits.push(UnitEditInstr::Delete {
        id: "missing".into(),
    });

    assert!(
        save_edits((&mock, &mock, &cache), token("translator-1"), invalid)
            .await
            .is_err()
    );

    assert!(cache.unit_save_token_count() == 0);

    assert!(mock.snapshot().units.is_empty());

    assert!(
        save_edits(
            (&mock, &mock, &cache),
            token("translator-1"),
            batch(&save_id, "text")
        )
        .await
        .is_ok()
    );
}

// A backend commit failure after successful steps must not publish a token.
#[tokio::test]
async fn failed_commit_does_not_publish_token() {
    let (mock, cache) = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let save_id = uuid::Uuid::new_v4().to_string();

    mock.set_commit_failure(true);

    assert!(
        save_edits(
            (&mock, &mock, &cache),
            token("translator-1"),
            batch(&save_id, "saved"),
        )
        .await
        .is_err()
    );

    assert_eq!(cache.unit_save_token_count(), 0);

    assert!(mock.snapshot().units.is_empty());

    mock.set_commit_failure(false);

    save_edits(
        (&mock, &mock, &cache),
        token("translator-1"),
        batch(&save_id, "saved"),
    )
    .await
    .unwrap();

    assert_eq!(mock.snapshot().units.len(), 1);

    assert_eq!(cache.unit_save_token_count(), 1);
}

#[tokio::test]
async fn delete_returns_empty_mapping_and_hides_unit() {
    let (mock, cache) = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    save_edits(
        (&mock, &mock, &cache),
        token("translator-1"),
        batch(&uuid::Uuid::new_v4().to_string(), "text"),
    )
    .await
    .unwrap();

    let deleted = save_edits(
        (&mock, &mock, &cache),
        token("translator-1"),
        save_instr(vec![UnitEditInstr::Delete {
            id: mock.snapshot().units.first().unwrap().id.clone(),
        }]),
    )
    .await
    .unwrap();

    assert!(deleted.created_unit_ids.is_empty());

    assert!(mock.snapshot().units.first().unwrap().hidden_at.is_some());

    assert_eq!(mock.snapshot().pages.first().unwrap().total_unit_count, 0);
}

// Concurrent retries share the committed result through independent port clones.
#[tokio::test]
async fn concurrent_retries_create_one_unit() {
    let (mock, cache) = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let cloned = mock.clone();

    let cloned_cache = cache.clone();

    let save_id = uuid::Uuid::new_v4().to_string();

    let (first, replay) = tokio::join!(
        save_edits(
            (&mock, &mock, &cache),
            token("translator-1"),
            batch(&save_id, "saved"),
        ),
        save_edits(
            (&cloned, &cloned, &cloned_cache),
            token("translator-1"),
            batch(&save_id, "saved"),
        ),
    );

    assert_eq!(first.unwrap(), replay.unwrap());

    assert_eq!(mock.snapshot().units.len(), 1);

    assert_eq!(cache.unit_save_token_count(), 1);
}

// Losing process-local tokens intentionally permits an old batch to run again.
#[tokio::test]
async fn lost_token_allows_reexecution() {
    let (mock, cache) = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    let save_id = uuid::Uuid::new_v4().to_string();

    save_edits(
        (&mock, &mock, &cache),
        token("translator-1"),
        batch(&save_id, "saved"),
    )
    .await
    .unwrap();

    let cache = MemCache::default();

    save_edits(
        (&mock, &mock, &cache),
        token("translator-1"),
        batch(&save_id, "saved"),
    )
    .await
    .unwrap();

    assert_eq!(mock.snapshot().units.len(), 2);
}

// A structural no-op still records its token and replays without touching Units.
#[tokio::test]
async fn empty_patch_records_token_without_refreshing_unit_timestamp() {
    let (mock, cache) = save_scope(RoleMask::from(RoleField::TRANSLATOR));

    save_edits(
        (&mock, &mock, &cache),
        token("translator-1"),
        batch(&uuid::Uuid::new_v4().to_string(), "saved"),
    )
    .await
    .unwrap();

    let unit_id = mock.snapshot().units.first().unwrap().id.clone();

    let original = time::OffsetDateTime::UNIX_EPOCH;

    mock.state
        .lock()
        .unwrap()
        .units
        .first_mut()
        .unwrap()
        .updated_at = original;

    let save_id = uuid::Uuid::new_v4().to_string();

    for _ in 0..2 {
        let mut instr = save_instr(vec![UnitEditInstr::Patch {
            id: unit_id.clone(),
            next_id: Patch::Skip,
            is_bubble: None,
            is_flagged: None,
            coord: None,
            typography_note: Patch::Skip,
            translation_note: Patch::Skip,
            translation: Patch::Skip,
            revision: Patch::Skip,
        }]);

        instr.save_id.clone_from(&save_id);

        save_edits((&mock, &mock, &cache), token("translator-1"), instr)
            .await
            .unwrap();

        assert_eq!(mock.snapshot().units.first().unwrap().updated_at, original);

        assert_eq!(cache.unit_save_token_count(), 2);
    }
}
