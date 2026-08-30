//! Upload-mark result values.

/// Result of marking one exact current object generation as uploaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkObjUploadedOutcome {
    //
    /// The supplied generation is current and now marked uploaded.
    Marked,

    /// The supplied generation is missing, stale, or detached.
    NotCurrent,
}
