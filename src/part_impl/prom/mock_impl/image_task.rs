pub(super) enum ResourceState {
    /// The image version matches the current DB record.
    Current,
    /// The image version is outdated; the resource has been superseded.
    Stale,
    /// The referenced resource no longer exists.
    Missing,
}
