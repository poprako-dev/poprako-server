#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use tokio::io::AsyncReadExt as _;

use super::*;

// string(ExportWriter)(positive): bounded UTF-8 fragments preserve JSON escaping across the fragment boundary.
#[tokio::test]
async fn string_fragments_preserve_unicode_and_control_characters() {
    let text = format!("{}😀\n\"\\\t\0中", "a".repeat(4095));

    let mut writer = ExportWriter::new(32 * 1024).unwrap();

    writer.string(&text).await.unwrap();

    let (mut file, bytes) = writer.finish().await.unwrap();

    let mut output = Vec::new();

    file.read_to_end(&mut output).await.unwrap();

    assert_eq!(output.len() as u64, bytes);

    assert_eq!(serde_json::from_slice::<String>(&output).unwrap(), text);
}

// write(ExportWriter)(negative): output beyond the artifact budget fails before exposing an artifact.
#[tokio::test]
async fn artifact_limit_counts_escaped_output() {
    let mut writer = ExportWriter::new(5).unwrap();

    let error = writer.string("\n\n").await.unwrap_err();

    assert!(matches!(error, BaseError::Unavailable { .. }));
}

// write(ExportWriter)(positive): exactly the configured artifact budget succeeds.
#[tokio::test]
async fn artifact_limit_accepts_exact_boundary() {
    let mut writer = ExportWriter::new(6).unwrap();

    writer.string("\n\n").await.unwrap();

    let (_, bytes) = writer.finish().await.unwrap();

    assert_eq!(bytes, 6);
}
