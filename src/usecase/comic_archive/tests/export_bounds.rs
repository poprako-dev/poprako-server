use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use tokio::io::AsyncReadExt as _;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::*;
use crate::model::read::proj::comic_archive::ComicArchiveRecord;
use crate::usecase::comic_archive::ComicArchiveExport;

const TEAM_ID: &str = "team--00000000001";

// Read completed JSON data in small regression fixtures.
async fn read_months(
    artifact: ComicArchiveExport,
) -> (BTreeMap<String, Vec<String>>, OwnedSemaphorePermit) {
    let (mut file, expected_bytes, permit) = artifact.into_transfer();

    let mut bytes = Vec::new();

    file.read_to_end(&mut bytes).await.unwrap();

    drop(file);

    assert_eq!(expected_bytes, bytes.len() as u64);

    (serde_json::from_slice(&bytes).unwrap(), permit)
}

// Seed immutable records without invoking unrelated archive mutation behavior.
fn seed_record(mock: &Mock, id: &str, month: &str, payload: String) {
    let months = ComicArchiveMonth::parse_retained(
        vec![month.into()],
        OffsetDateTime::now_utc(),
    )
    .unwrap();

    mock.state
        .lock()
        .unwrap()
        .comic_archives
        .push(ComicArchiveRecord {
            id: id.into(),
            team_id: TEAM_ID.into(),
            source_comic_id: format!("source-{id}"),
            archived_payload: payload,
            archiver_id: USER_ID.into(),
            created_at: months.first().unwrap().start,
        });
}

// export(export)(positive): stored JSON strings retain their exact contents.
#[tokio::test]
async fn export_returns_stored_strings_grouped_by_month() {
    let mock = Mock::new();

    seed_archive_scope(&mock, RoleMask::from(RoleField::ADMIN));

    archive((&mock, &mock, &mock), token(), COMIC_ID.into())
        .await
        .unwrap();

    let now = OffsetDateTime::now_utc();

    let month = format!("{:04}-{:02}", now.year(), u8::from(now.month()));

    let config = ComicArchiveExportConfig::default();

    let admission = Arc::new(Semaphore::new(1));

    let (months, permit) = read_months(
        export(
            (&mock, &mock, &config, &admission),
            token(),
            TEAM_ID.into(),
            ExportComicArchivesInstr {
                months: vec![month.clone()],
            },
        )
        .await
        .unwrap(),
    )
    .await;

    drop(permit);

    let snapshot = mock.snapshot();

    assert_eq!(
        months.get(&month).unwrap(),
        &vec![
            snapshot
                .comic_archives
                .first()
                .unwrap()
                .archived_payload
                .clone()
        ]
    );
}

// export(export)(positive): tied timestamps traverse multiple metadata batches without duplicates, gaps, or lost empty months.
#[tokio::test]
async fn export_preserves_complete_selected_months_across_chunks() {
    let mock = Mock::new();

    seed_archive_scope(&mock, RoleMask::from(RoleField::ADMIN));

    for index in (0..65).rev() {
        seed_record(
            &mock,
            &format!("archive-{index:03}"),
            "2025-01",
            format!("payload-{index:03}\n\"中"),
        );
    }

    seed_record(&mock, "unselected", "2025-02", "unselected".into());

    let config = ComicArchiveExportConfig::default();

    let admission = Arc::new(Semaphore::new(1));

    let (months, permit) = read_months(
        export(
            (&mock, &mock, &config, &admission),
            token(),
            TEAM_ID.into(),
            ExportComicArchivesInstr {
                months: vec!["2025-03".into(), "2025-01".into()],
            },
        )
        .await
        .unwrap(),
    )
    .await;

    assert_eq!(months.len(), 2);

    assert_eq!(
        months.get("2025-01").unwrap(),
        &(0..65)
            .map(|index| format!("payload-{index:03}\n\"中"))
            .collect::<Vec<_>>()
    );

    assert!(months.get("2025-03").unwrap().is_empty());

    assert_eq!(admission.available_permits(), 0);

    drop(permit);

    assert_eq!(admission.available_permits(), 1);
}

// export(export)(negative): resource rejection and failed authorization never consume permanent admission capacity.
#[tokio::test]
async fn export_failure_releases_admission_and_preserves_permission_errors() {
    let mock = Mock::new();

    seed_archive_scope(&mock, RoleMask::from(RoleField::ADMIN));

    seed_record(&mock, "oversized", "2025-01", "x".repeat(1024 * 1024 + 1));

    let config = ComicArchiveExportConfig {
        payload_batch_mib: NonZeroU32::MIN,
        ..ComicArchiveExportConfig::default()
    };

    let admission = Arc::new(Semaphore::new(1));

    let error = export(
        (&mock, &mock, &config, &admission),
        token(),
        TEAM_ID.into(),
        ExportComicArchivesInstr {
            months: vec!["2025-01".into()],
        },
    )
    .await
    .err()
    .unwrap();

    assert!(matches!(error, BaseError::Unavailable { .. }));

    assert_eq!(admission.available_permits(), 1);

    let permit = admission.clone().try_acquire_owned().unwrap();

    let error = export(
        (&mock, &mock, &config, &admission),
        token(),
        TEAM_ID.into(),
        ExportComicArchivesInstr {
            months: vec!["2025-01".into()],
        },
    )
    .await
    .err()
    .unwrap();

    assert!(matches!(error, BaseError::Unavailable { .. }));

    let error = export(
        (&mock, &mock, &config, &admission),
        token(),
        "another-team".into(),
        ExportComicArchivesInstr {
            months: vec!["2025-01".into()],
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);

    drop(permit);

    let (months, permit) = read_months(
        export(
            (&mock, &mock, &config, &admission),
            token(),
            TEAM_ID.into(),
            ExportComicArchivesInstr {
                months: vec!["2025-03".into()],
            },
        )
        .await
        .unwrap(),
    )
    .await;

    drop(permit);

    assert!(months.get("2025-03").unwrap().is_empty());
}
