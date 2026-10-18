//! Integer-second deserialization for runtime scheduling policies.

use std::time::{Duration, Instant};

use serde::de::Error as _;
use serde::{Deserialize as _, Deserializer};

/// Reads seconds that can be added to the process clock without overflow.
///
/// # Errors
/// Rejects non-integer values and durations exceeding the process clock range.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    //
    let seconds = u64::deserialize(deserializer)?;

    let duration = Duration::from_secs(seconds);

    if Instant::now().checked_add(duration).is_none() {
        //
        return Err(D::Error::custom(
            "duration exceeds the process clock range",
        ));
    }

    Ok(duration)
}
