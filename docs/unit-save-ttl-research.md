# Unit save token expiration in Moka

Investigated on 2026-10-18 against the dependency locked in `Cargo.lock`, Moka
0.12.16. The versioned upstream source was checked alongside the downloaded
crate source. The recommendation is a cache-level TTL of 30 days
(`2_592_000` seconds), without a custom per-entry `Expiry` implementation.

## Expiration and timer accuracy

Cache-level TTL expires an entry after its most recent insertion or replacement.
Reading it does not refresh TTL; TTI would refresh on reads. Consequently,
replaying a save token through `get` does not extend its retention period.
[Official expiration documentation](https://docs.rs/moka/0.12.16/moka/sync/struct.Cache.html#cache-level-ttl-and-tti-policies).

The save use case previously called `record` again after successful replay,
which would restart Moka's TTL despite the lookup itself being read-only. The
implementation now records only a newly committed save; a retained replay returns
the existing mapping without reinsertion. The default is configured through
`cache.unit_save_ttl_secs`.
[Save use case](../src/usecase/unit.rs),
[cache configuration](../src/config/cache.rs).

The synchronous lookup path checks `last_modified + ttl <= now` before returning
the value. An entry that has reached its deadline is a miss even if it remains
allocated. This check uses the time sampled for that lookup, so a lookup started
before the deadline can finish afterward; this is not a real-time response
deadline. Fixed TTL cleanup uses a write-order deque. The timer wheel is enabled
only when per-entry expiration timestamps exist, which the plain TTL policy does
not create. **There is no timer-wheel rounding of the 30-day token validity.**
[Lookup and expiry checks](https://github.com/moka-rs/moka/blob/v0.12.16/src/sync/base_cache.rs#L273-L309),
[TTL comparison](https://github.com/moka-rs/moka/blob/v0.12.16/src/sync/base_cache.rs#L2621-L2632),
[deque cleanup](https://github.com/moka-rs/moka/blob/v0.12.16/src/sync/base_cache.rs#L2024-L2045),
[wheel activation](https://github.com/moka-rs/moka/blob/v0.12.16/src/sync/base_cache.rs#L1809-L1834).

Moka's expiry clock reads elapsed `std::time::Instant`; internal timestamps retain
nanoseconds. This avoids calendar-time or time-zone rounding, but nanosecond
representation does not promise nanosecond operating-system accuracy. The TTL
means elapsed duration as measured by that clock, not a calendar month.
[Clock](https://github.com/moka-rs/moka/blob/v0.12.16/src/common/time/clock.rs#L71-L88),
[timestamp representation](https://github.com/moka-rs/moka/blob/v0.12.16/src/common/time/instant.rs#L1-L49).

## Physical removal can lag

Housekeeping is triggered by cache operations when operation-log thresholds are
reached or the maintenance eligibility time has passed. In this version that
interval is **300 ms**; it is not a periodic background timer or a maximum removal
delay. Automatic maintenance uses a try-lock. With no operations and no explicit
maintenance, expired entries can remain allocated indefinitely. This does not
make them readable again.
[Housekeeper](https://github.com/moka-rs/moka/blob/v0.12.16/src/common/concurrent/housekeeper.rs#L78-L131),
[interval constant](https://github.com/moka-rs/moka/blob/v0.12.16/src/common/concurrent/constants.rs#L1-L2).

`SegmentedCache` gives each segment the expiration policy. Operations target the
selected segment, while `run_pending_tasks()` visits every segment. An idle
segment can therefore retain expired allocations even while other segments are
active. Periodic application maintenance would be needed only if timely memory
reclamation during inactivity becomes a requirement; lazy cleanup is sufficient
for the requested token-validity contract.
[Segment construction](https://github.com/moka-rs/moka/blob/v0.12.16/src/sync/segment.rs#L705-L746),
[explicit maintenance](https://github.com/moka-rs/moka/blob/v0.12.16/src/sync/segment.rs#L625-L630).

## The separate per-entry timer wheel

Moka does contain a hierarchical timer wheel for variable per-entry expiration.
Its spans are approximately 1.07 seconds, 1.14 minutes, 1.22 hours, and 1.63 days,
with an overflow queue beyond approximately 6.5 days. These are scheduling levels,
not permission to serve entries past their deadlines: wheel advancement checks
actual timestamps and reschedules entries that are not expired, while lookups
also check per-entry timestamps directly. Those large spans must not be reported
as a multi-day validity error for a fixed 30-day TTL.
[Wheel spans and advancement](https://github.com/moka-rs/moka/blob/v0.12.16/src/common/timer_wheel.rs#L24-L44),
[timestamp checks and rescheduling](https://github.com/moka-rs/moka/blob/v0.12.16/src/common/timer_wheel.rs#L549-L649),
[per-entry lookup check](https://github.com/moka-rs/moka/blob/v0.12.16/src/sync/base_cache.rs#L2525-L2533).

Thirty days is a maximum retention age, not a guaranteed replay window: capacity
eviction can remove tokens earlier, and process-local storage does not survive
restart. This follows from the existing bounded in-memory
[Unit save cache](../src/part_impl/repo/mem_impl/unit_save.rs) and Moka's
[size-based eviction contract](https://docs.rs/moka/0.12.16/moka/sync/struct.Cache.html#example-size-based-eviction).
