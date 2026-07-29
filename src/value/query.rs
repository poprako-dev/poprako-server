//! Custom serde helpers for axum query-string extraction.
//!
//! `serde_urlencoded` (the backend of `axum::extract::Query`) yields individual
//! key-value pairs one at a time and **does not** group repeated keys.  This
//! means `?incl=a&incl=b` produces two separate map entries for the `incl`
//! field, which triggers `duplicate field` errors when a custom
//! `#[serde(deserialize_with)]` is used on a `Vec<T>` field.
//!
//! The helpers in this module work around the limitation by **grouping** values
//! by key before deserializing.  [`GroupedQuery`] is a drop-in replacement for
//! `axum::extract::Query` that groups repeated keys first.
//!
//! # How grouping works
//!
//! 1. Parse the raw query string with `url::form_urlencoded::parse`.
//! 2. Group values by key — `{"incl": ["a", "b"]}`.
//! 3. Convert to `serde_json::Value` (single → string, multi → array).
//! 4. Deserialize the struct via `serde_json::from_value`.
//!
//! The existing [`deserialize_vec`] custom deserialiser handles both the single-
//! string and array shapes through `visit_str` / `visit_seq`.

use std::fmt;
use std::marker::PhantomData;

use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use serde::Deserialize as _;
use serde::de::{self, DeserializeOwned, Deserializer, SeqAccess, Visitor};

#[cfg(test)]
mod tests;

/// Axum request-extractor that groups repeated query-string keys before
/// deserialising the target struct.
///
/// Same interface as `axum::extract::Query<T>` but handles repeated parameters
/// such as `?incl=a&incl=b` without triggering `duplicate field` errors.
///
/// # Example
///
/// ```ignore
/// async fn list_comics(
///     GroupedQuery(query): GroupedQuery<ComicListQuery>,
/// ) -> HttpResult<...> {
///     // query.incl_opt  — contains both "a" and "b"
///     // query.with_opt  — contains both "c" and "d"
/// }
/// ```
pub struct GroupedQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for GroupedQuery<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        //
        let query_str = parts.uri.query().unwrap_or_default();

        if query_str.is_empty() {
            return serde_json::from_value(serde_json::Value::Object(
                serde_json::Map::new(),
            ))
            .map(GroupedQuery)
            .map_err(|e| {
                (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    format!("Failed to deserialize query string: {}", e),
                )
            });
        }

        let value = from_grouped_query::<T>(query_str).map_err(|e| {
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("Failed to deserialize query string: {}", e),
            )
        })?;

        Ok(GroupedQuery(value))
    }
}

/// Deserialise `T` from a URL-encoded query string, grouping repeated keys.
///
/// The input `?key=a&key=b` is grouped into `{"key": ["a", "b"]}` before
/// deserialisation so that `Vec<T>` fields populated via
/// [`deserialize_vec`] receive a proper sequence.
pub fn from_grouped_query<T: DeserializeOwned>(
    input: &str,
) -> Result<T, String> {
    //
    use std::collections::HashMap;

    let mut groups: HashMap<String, Vec<String>> = HashMap::new();

    for (key, value) in url::form_urlencoded::parse(input.as_bytes()) {
        groups
            .entry(key.into_owned())
            .or_default()
            .push(value.into_owned());
    }

    let mut obj = serde_json::Map::with_capacity(groups.len());

    for (key, values) in groups {
        //
        let json_values: Vec<serde_json::Value> = values
            .iter()
            .map(|v| {
                serde_json::from_str(v)
                    .unwrap_or_else(|_| serde_json::Value::String(v.clone()))
            })
            .collect();

        let entry = if json_values.len() == 1 {
            json_values.into_iter().next().unwrap()
        } else {
            serde_json::Value::Array(json_values)
        };

        obj.insert(key, entry);
    }

    serde_json::from_value(serde_json::Value::Object(obj))
        .map_err(|e| format!("{}", e))
}

/// Deserialise `Vec<T>` from a query-string value that may be a **single
/// string** or a **repeated key** (sequence).
///
/// # Usage
///
/// ```ignore
/// #[serde(default, deserialize_with = "crate::value::query::deserialize_vec")]
/// pub incl_opt: Vec<MemberInclOpt>,
/// ```
pub fn deserialize_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned,
{
    struct VecVisitor<T>(PhantomData<T>);

    impl<'de, T> Visitor<'de> for VecVisitor<T>
    where
        T: DeserializeOwned,
    {
        type Value = Vec<T>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a single value or a repeated query parameter")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            //
            let item = deserialize_from_str::<T, E>(v)?;

            Ok(vec![item])
        }

        fn visit_string<E: de::Error>(
            self,
            v: String,
        ) -> Result<Self::Value, E> {
            //
            let item = deserialize_from_str::<T, E>(&v)?;

            Ok(vec![item])
        }

        fn visit_seq<S: SeqAccess<'de>>(
            self,
            seq: S,
        ) -> Result<Self::Value, S::Error> {
            Vec::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }
    }

    deserializer.deserialize_any(VecVisitor(PhantomData))
}

/// Deserialise `T` from a string slice via `serde_json::Value`, which owns its
/// data and therefore implements `Deserializer<'de>` for every `'de`.
/// Deserialize `T` from a string slice via a serde_json value round-trip.
///
/// This bypasses lifetime restrictions: `serde_json::Value::String` owns its
/// data, so it implements `Deserializer<'de>` for every `'de`, allowing
/// deserialization of owned types that cannot borrow from the original
/// query-string slice.
fn deserialize_from_str<T, E>(s: &str) -> Result<T, E>
where
    T: DeserializeOwned,
    E: de::Error,
{
    serde_json::from_value(serde_json::Value::String(s.to_owned()))
        .map_err(de::Error::custom)
}
