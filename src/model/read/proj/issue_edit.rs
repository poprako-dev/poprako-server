//! Consistent review versions and category usage.

/// Chapter-wide review identity and mutation ver.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IssueEditReviewVer {
    /// Whole-review replacement generation.
    pub gen_no: i64,
    /// Version of the chapter's review context.
    pub revision: i64,
}

/// One category actually referenced by stored annotations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueEditVariantCount {
    /// Normalized category name.
    pub variant: String,
    /// Number of current annotations referencing it.
    pub count: i64,
}
