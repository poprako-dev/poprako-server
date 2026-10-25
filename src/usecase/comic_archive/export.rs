//! Incremental complete JSON export construction and temporary-file ownership.

#[cfg(test)]
mod tests;

use std::io::{Error, SeekFrom};

use tokio::fs::File;
use tokio::io::{AsyncSeekExt as _, AsyncWriteExt as _, BufWriter};

use crate::result::{BaseError, BaseRest};

/// Writes a complete export without retaining escaped copies of entire payloads.
pub struct ExportWriter {
    /// Buffered anonymous temporary file, deleted automatically on drop.
    file: BufWriter<File>,
    /// Serialized bytes written so far.
    bytes: u64,
    /// Maximum permitted serialized output bytes.
    limit: u64,
}

impl ExportWriter {
    /// Creates an anonymous artifact with a finite serialized byte budget.
    ///
    /// # Errors
    /// Returns an infrastructure error when temporary file creation fails.
    pub fn new(limit: u64) -> BaseRest<Self> {
        //
        let file = tempfile::tempfile().map_err(|error| io_error(&error))?;

        Ok(Self {
            file: BufWriter::with_capacity(32 * 1024, File::from_std(file)),
            bytes: 0,
            limit,
        })
    }

    /// Writes punctuation or one bounded escaped string fragment.
    ///
    /// # Errors
    /// Returns an error when the byte budget is exceeded or a file write fails.
    pub async fn write(&mut self, bytes: &[u8]) -> BaseRest<()> {
        //
        let next = self.bytes.saturating_add(bytes.len() as u64);

        if next > self.limit {
            //
            tracing::warn!(
                operation = "export_comic_archives",
                "archive export exceeds temporary artifact budget"
            );

            return Err(BaseError::Unavailable {
                message: "archive export artifact budget exceeded".into(),
            });
        }

        self.file
            .write_all(bytes)
            .await
            .map_err(|error| io_error(&error))?;

        self.bytes = next;

        Ok(())
    }

    /// Encodes small UTF-8 slices without multiplying a full payload batch.
    ///
    /// # Errors
    /// Returns an error for encoding, artifact budget, or file write failures.
    pub async fn string(&mut self, mut text: &str) -> BaseRest<()> {
        //
        self.write(b"\"").await?;

        while !text.is_empty() {
            //
            let end = text.floor_char_boundary(text.len().min(4096));

            let Some((fragment, remaining)) = text.split_at_checked(end) else {
                //
                return Err(unrecoverable_error! {
                    message: "archive export UTF-8 boundary is invalid".into();
                    "archive export encoding invariant failed",
                });
            };

            let escaped = serde_json::to_vec(fragment).map_err(|error| {
                //
                unrecoverable_error! {
                    message: "archive export string encoding failed".into();
                    sdk_error = ?error,
                    "JSON SDK archive export encoding error",
                }
            })?;

            let Some(fragment) = escaped
                .strip_prefix(b"\"")
                .and_then(|bytes| bytes.strip_suffix(b"\""))
            else {
                //
                return Err(unrecoverable_error! {
                    message: "archive export string delimiters missing".into();
                    "archive export encoding invariant failed",
                });
            };

            self.write(fragment).await?;

            text = remaining;
        }

        self.write(b"\"").await
    }

    /// Flushes and rewinds before any response headers are sent.
    ///
    /// # Errors
    /// Returns an infrastructure error when flushing or seeking fails.
    pub async fn finish(mut self) -> BaseRest<(File, u64)> {
        //
        self.file.flush().await.map_err(|error| io_error(&error))?;

        let mut file = self.file.into_inner();

        file.seek(SeekFrom::Start(0))
            .await
            .map_err(|error| io_error(&error))?;

        Ok((file, self.bytes))
    }
}

// Records external file failures once before returning an infrastructure error.
fn io_error(error: &Error) -> BaseError {
    //
    unrecoverable_error! {
        message: "archive export temporary file operation failed".into();
        sdk_error = ?error,
        "file SDK archive export error",
    }
}
