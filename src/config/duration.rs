//! String duration decoding for runtime policies.
use std::time::{Duration, Instant};

use serde::de::Error as _;
use serde::{Deserialize as _, Deserializer};

/// Reads a unit-bearing duration string such as `300s`, `5m` or `30d`.
///
/// # Errors
/// Rejects non-string values, durations without a unit, malformed durations
/// and durations exceeding the process clock range.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    //
    let text = String::deserialize(deserializer)?;

    if text.chars().all(|character| !character.is_alphabetic()) {
        //
        return Err(D::Error::custom(
            "duration must include a unit such as s, m, h or d",
        ));
    }

    let duration = fundu::parse_duration(&text).map_err(D::Error::custom)?;

    if Instant::now().checked_add(duration).is_none() {
        //
        return Err(D::Error::custom(
            "duration exceeds the process clock range",
        ));
    }

    Ok(duration)
}
