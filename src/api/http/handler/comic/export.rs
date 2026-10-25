//! Streaming transport for completed archive export artifacts.

#[cfg(test)]
mod tests;

use std::io::Error;

use axum::body::{Body, Bytes};
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use axum::response::{IntoResponse as _, Response};
use futures_util::{StreamExt as _, stream};
use tokio_util::io::ReaderStream;

use crate::usecase::comic_archive::ComicArchiveExport;

// The success envelope remains a transport concern rather than a use-case input.
const PREFIX: &[u8] = b"{\"code\":0,\"data\":";

/// Streams a completed artifact, retaining admission through completion or drop.
pub fn response(artifact: ComicArchiveExport) -> Response {
    //
    let (file, bytes, permit) = artifact.into_transfer();

    let length = bytes + PREFIX.len() as u64 + 1;

    let reader = ReaderStream::with_capacity(file, 32 * 1024);

    let body = stream::unfold(
        (reader, permit, 0_u8),
        |(mut reader, permit, phase)| async move {
            //
            match phase {
                //
                0 => Some((
                    Ok::<Bytes, Error>(Bytes::from_static(PREFIX)),
                    (reader, permit, 1),
                )),

                1 => match reader.next().await {
                    //
                    Some(Ok(bytes)) => Some((Ok(bytes), (reader, permit, 1))),

                    Some(Err(error)) => {
                        //
                        tracing::error!(
                            operation = "stream_comic_archive_export",
                            sdk_error = ?error,
                            "file SDK archive export read error",
                        );

                        Some((Err(error), (reader, permit, 2)))
                    }

                    None => Some((
                        Ok(Bytes::from_static(b"}")),
                        (reader, permit, 2),
                    )),
                },

                _ => None,
            }
        },
    );

    (
        [
            (CONTENT_TYPE, "application/json".to_owned()),
            (CONTENT_LENGTH, length.to_string()),
        ],
        Body::from_stream(body),
    )
        .into_response()
}
