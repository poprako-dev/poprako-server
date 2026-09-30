//! HTTP request extraction with warning-level rejection diagnostics.

#[cfg(test)]
mod tests;

use std::any::type_name;
use std::error::Error;

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum_extra::extract::QueryRejection as MultiQueryRejection;
use serde::de::DeserializeOwned;

// Remove quoted input values while preserving missing schema-field diagnostics.
fn redact_diagnostic(message: &str) -> String {
    //
    // These error forms may interpolate unescaped client text, including delimiters.
    let sensitive = [
        "unknown variant ",
        "unknown field ",
        "invalid type: string ",
        "invalid value: string ",
        "Cannot parse ",
    ]
    .into_iter()
    .filter_map(|marker| message.find(marker).map(|index| (index, marker)))
    .min_by_key(|(index, _)| *index);

    if let Some((index, marker)) = sensitive {
        //
        let prefix = message.get(..index).unwrap_or_default();

        return format!("{}{}<REDACTED>", prefix, marker);
    }

    let mut safe = String::new();

    let mut chars = message.chars();

    while let Some(character) = chars.next() {
        //
        if !matches!(character, '"' | '`') {
            //
            safe.push(character);

            continue;
        }

        let preserve = character == '`'
            && (safe.ends_with("missing field ")
                || safe.ends_with("duplicate field ")
                || safe.ends_with("Expected request with "));

        safe.push(character);

        if !preserve {
            safe.push_str("<REDACTED>");
        }

        while let Some(quoted) = chars.next() {
            //
            if quoted == character {
                //
                safe.push(quoted);

                break;
            }

            if preserve {
                safe.push(quoted);
            }

            if quoted == '\\' {
                let _ = chars.next();
            }
        }
    }

    safe
}

// Record the actual extractor rejection before Axum converts it to a response.
fn record_rejection<E>(source: &E, status: StatusCode)
where
    E: Error,
{
    //
    if !status.is_client_error() {
        return;
    }

    let err_message = redact_diagnostic(&source.to_string());

    let mut causes = Vec::new();

    let mut cause = source.source();

    while let Some(error) = cause {
        //
        causes.push(redact_diagnostic(&error.to_string()));

        cause = error.source();
    }

    let err_source = causes.join(": ");

    tracing::warn!(
        status = status.as_u16(),
        err_variant = type_name::<E>(),
        err_message,
        err_source,
        "HTTP request extraction rejected",
    );
}

/// JSON request body extraction preserving Axum's rejection responses.
pub struct Json<T>(pub T);

impl<S, T> FromRequest<S> for Json<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    // Preserve the upstream extractor rejection type.
    type Rejection = JsonRejection;

    // Extract the body and record rejected requests.
    async fn from_request(
        request: Request,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        //
        axum::Json::<T>::from_request(request, state)
            .await
            .map(|axum::Json(value)| Self(value))
            .inspect_err(|source| record_rejection(source, source.status()))
    }
}

/// URL path extraction preserving Axum's rejection responses.
pub struct Path<T>(pub T);

impl<S, T> FromRequestParts<S> for Path<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    // Preserve the upstream extractor rejection type.
    type Rejection = PathRejection;

    // Extract request parts and record rejected requests.
    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        //
        axum::extract::Path::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Path(value)| Self(value))
            .inspect_err(|source| record_rejection(source, source.status()))
    }
}

/// Query extraction preserving Axum's single-value query semantics.
pub struct Query<T>(pub T);

impl<S, T> FromRequestParts<S> for Query<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    // Preserve the upstream extractor rejection type.
    type Rejection = QueryRejection;

    // Extract request parts and record rejected requests.
    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        //
        axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Query(value)| Self(value))
            .inspect_err(|source| record_rejection(source, source.status()))
    }
}

/// Query extraction preserving axum-extra's repeated-key query semantics.
#[cfg_attr(test, derive(Debug))]
pub struct MultiQuery<T>(pub T);

#[cfg(test)]
impl<T> MultiQuery<T>
where
    T: DeserializeOwned,
{
    /// Retain URI parsing for handler contract tests without an HTTP request.
    pub fn try_from_uri(
        uri: &axum::http::Uri,
    ) -> Result<Self, MultiQueryRejection> {
        //
        axum_extra::extract::Query::<T>::try_from_uri(uri)
            .map(|axum_extra::extract::Query(value)| Self(value))
            .inspect_err(|source| record_rejection(source, source.status()))
    }
}

impl<S, T> FromRequestParts<S> for MultiQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    // Preserve the upstream extractor rejection type.
    type Rejection = MultiQueryRejection;

    // Extract request parts and record rejected requests.
    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        //
        axum_extra::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map(|axum_extra::extract::Query(value)| Self(value))
            .inspect_err(|source| record_rejection(source, source.status()))
    }
}
