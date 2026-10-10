//! Authoritative composite page manifests.

/// Final identity and metadata for one composite page.
pub struct PageArtworkEntry {
    /// Stable page identity.
    pub id: String,

    /// Owning Chapter.
    pub chapter_id: String,
    /// Zero-based position.
    pub index: usize,

    /// Original PSD filename or relative path.
    pub raw_ident: Option<String>,
}

/// Mutable presentation metadata for one composite page.
pub struct PageArtworkPatch {
    /// Stable page identity.
    pub id: String,

    /// New original PSD filename or relative path; None clears it.
    pub raw_ident: Option<String>,
}
