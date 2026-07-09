//! Custom serde helpers for axum query-string extraction.
//!
//! `serde_urlencoded` (the backend of `axum::extract::Query`) deserialises a
//! repeated key (`?incl=team&incl=user`) as a sequence but rejects a single
//! occurrence (`?incl=team`) for `Vec<T>` fields.  The helpers in this module
//! accept both shapes so that callers don't have to repeat query params.

use std::fmt;
use std::marker::PhantomData;

use serde::Deserialize as _;
use serde::de::{self, DeserializeOwned, Deserializer, SeqAccess, Visitor};

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
fn deserialize_from_str<T, E>(s: &str) -> Result<T, E>
where
    T: DeserializeOwned,
    E: de::Error,
{
    serde_json::from_value(serde_json::Value::String(s.to_owned()))
        .map_err(de::Error::custom)
}
