/// Maximum number of source pages allowed in one Chapter.
pub const MAX_CHAPTER_PAGE_SOURCE_COUNT: usize = 200;

/// Database sentinel used to detect a Chapter exceeding the source page maximum.
pub const CHAPTER_PAGE_SOURCE_SENTINEL_LIMIT: i64 = 201;

/// Business fields used to match an existing page to an incoming image.
pub struct PageSourceManifestCand<'a> {
    /// Stable page identity.
    pub id: &'a str,

    /// Owning chapter identity.
    pub chapter_id: &'a str,
}

/// One manifest position and the old page assigned to it, when any.
pub struct ManifestMatch {
    /// Index of the matched existing page, or none for a new page.
    pub existing_index: Option<usize>,
}

/// Stable matching result for an authoritative page manifest.
pub struct ManifestPlan {
    /// Ordered match results aligned with the requested manifest.
    pub matches: Vec<ManifestMatch>,
    /// Existing page indexes not consumed by the requested manifest.
    pub deleted_existing_indexes: Vec<usize>,
}
