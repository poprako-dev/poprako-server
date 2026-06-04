use time::OffsetDateTime;

/// Converts a timestamp to Unix epoch milliseconds.
///
/// Implemented for [`OffsetDateTime`](time::OffsetDateTime) to produce the
/// integer millisecond representation expected by frontend clients.
pub trait ToUnixMilli {
    /// Returns the number of milliseconds since 1970-01-01T00:00:00Z.
    fn to_unix_milli(&self) -> i64;
}

impl ToUnixMilli for OffsetDateTime {
    fn to_unix_milli(&self) -> i64 {
        self.unix_timestamp() * 1000 + (self.nanosecond() / 1_000_000) as i64
    }
}
