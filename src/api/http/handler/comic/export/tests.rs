#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use std::io::Write as _;
use std::sync::Arc;

use axum::body::to_bytes;
use futures_util::StreamExt as _;
use tokio::sync::Semaphore;

use super::*;

// response(ComicArchiveExport)(positive): the exact success envelope retains strings and empty months.
#[tokio::test]
async fn response_preserves_wire_envelope_and_releases_admission() {
    let admission = Arc::new(Semaphore::new(1));

    let data = br#"{"2025-01":["{\"title\":\"sample\"}"],"2025-03":[]}"#;

    let mut file = tempfile::tempfile().unwrap();

    file.write_all(data).unwrap();

    std::io::Seek::rewind(&mut file).unwrap();

    let response = response(ComicArchiveExport::new(
        tokio::fs::File::from_std(file),
        data.len() as u64,
        admission.clone().try_acquire_owned().unwrap(),
    ));

    assert_eq!(admission.available_permits(), 0);

    assert_eq!(
        response.headers().get(CONTENT_TYPE).unwrap(),
        "application/json"
    );

    let body = to_bytes(response.into_body(), 1024).await.unwrap();

    let expected = [PREFIX, data, b"}"].concat();

    assert_eq!(body.as_ref(), expected);

    assert_eq!(admission.available_permits(), 1);
}

// response(ComicArchiveExport)(negative): dropping a partially consumed body releases admission immediately.
#[tokio::test]
async fn dropped_response_releases_admission() {
    let admission = Arc::new(Semaphore::new(1));

    let response = response(ComicArchiveExport::new(
        tokio::fs::File::from_std(tempfile::tempfile().unwrap()),
        0,
        admission.clone().try_acquire_owned().unwrap(),
    ));

    assert_eq!(admission.available_permits(), 0);

    let mut stream = response.into_body().into_data_stream();

    assert_eq!(stream.next().await.unwrap().unwrap().as_ref(), PREFIX);

    assert_eq!(admission.available_permits(), 0);

    drop(stream);

    assert_eq!(admission.available_permits(), 1);
}
