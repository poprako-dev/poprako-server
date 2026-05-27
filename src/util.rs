pub mod time {
    use time::OffsetDateTime;

    pub trait ToUnixMilli {
        fn to_unix_milli(&self) -> i64;
    }

    impl ToUnixMilli for OffsetDateTime {
        fn to_unix_milli(&self) -> i64 {
            self.unix_timestamp() * 1000 + (self.nanosecond() / 1_000_000) as i64
        }
    }
}

pub mod rename {
    pub type StdResl<T, E> = std::result::Result<T, E>;
}
